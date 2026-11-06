#!/usr/bin/env bash
# One-off: move the live box from echo-prod-2026-11-06 to the upstream merge with echo as a
# naming strategy. Ours, not upstream's; delete it once the box has been through it.
#
# Run from a checkout, on the box over ssh:
#
#     ssh ovhwormhole 'bash -s' < deploy/echo-upgrade.sh            # deploy origin/echo
#     ssh ovhwormhole 'bash -s -- deploy <ref>' < deploy/echo-upgrade.sh
#     ssh ovhwormhole 'bash -s -- rollback <dump>' < deploy/echo-upgrade.sh
#
# Why it is not just `git pull && docker compose up`:
#
# - Our migration was 0041 and is now 0046, behind upstream's 0041-0045. The database
#   records it as version 41, so that row is renumbered before the new API starts, or the
#   API refuses to boot. Once renumbered, the old API refuses to boot instead, so the old
#   API is stopped first and the new one started straight after.
# - Upstream's 0042 renames the sessions column and deletes every session. The old images
#   cannot run on the new schema, so going back means restoring the dump taken here.
# - Everyone has to sign in again, which needs ESI. The script will not start while ESI is
#   down.
set -Eeuo pipefail

# Overridable for a rehearsal on a copy of the stack.
cd "${WS_DIR:-$HOME/wormholesystems}"

ROLLBACK_TAG=echo-prod-2026-11-06
OLD_MIGRATION="alias scheme echo"
SITE=${WS_SITE:-https://wormhole.3cho.space}
BACKUPS=${WS_BACKUPS:-$HOME/backups}

# Set once the old API can no longer start on this database; every stop after that says
# how to go back.
DUMP_FOR_ROLLBACK=""

say() { printf '\n== %s\n' "$*"; }
die() {
	printf '\nSTOPPED: %s\n' "$*" >&2
	if [ -n "$DUMP_FOR_ROLLBACK" ]; then
		printf 'The old images cannot start on this database any more. To go back:\n' >&2
		printf "  ssh ovhwormhole 'bash -s -- rollback %s' < deploy/echo-upgrade.sh\n" "$DUMP_FOR_ROLLBACK" >&2
	fi
	exit 1
}
trap 'die "a command failed (line $LINENO)"' ERR
psql_db() { docker compose exec -T db psql -U wormholesystems -d wormholesystems -v ON_ERROR_STOP=1 -Atc "$1"; }

deploy() {
	local ref=${1:-origin/echo}

	say "Checks before anything changes"
	mkdir -p "$BACKUPS"
	[ "$(git branch --show-current)" = echo ] || die "the checkout is not on branch echo"
	[ -z "$(git status --porcelain --untracked-files=no)" ] || die "the checkout has local changes"
	git fetch --quiet origin --tags
	git merge-base --is-ancestor HEAD "$ref" || die "$ref does not contain the running commit $(git rev-parse --short HEAD)"
	local esi
	esi=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 https://esi.evetech.net/status/) || true
	[ "$esi" = 200 ] || die "ESI answers $esi; nobody could sign back in after the deploy"
	local free_gb
	free_gb=$(df --output=avail -BG / | tail -1 | tr -dc 0-9)
	[ "$free_gb" -ge 8 ] || die "only ${free_gb} GB free; the build needs about 4"
	local version
	version=$(psql_db "select version from _sqlx_migrations where description = '$OLD_MIGRATION'")
	case "$version" in
	41) ;;
	46) echo "the migration row is already 46: an earlier run got that far" ;;
	*) die "expected '$OLD_MIGRATION' at version 41 or 46, found '${version:-nothing}'" ;;
	esac
	local echo_maps
	echo_maps=$(psql_db "select count(*) from maps where alias_scheme = 'echo'")
	echo "running $(git rev-parse --short HEAD), deploying $(git rev-parse --short "$ref"); $echo_maps echo maps"

	say "Build the new images while the old ones keep serving"
	local running
	running=$(git rev-parse HEAD)
	git merge --quiet --ff-only "$ref"
	if ! docker compose build --pull api web >"$BACKUPS/build-$(date -u +%Y%m%d-%H%M%S).log" 2>&1; then
		git reset --quiet --hard "$running"
		die "the build failed (log in $BACKUPS); the checkout is back on $(git rev-parse --short HEAD) and nothing else changed"
	fi

	say "Dump the database"
	local dump
	dump="$BACKUPS/pre-upgrade-$(date -u +%Y%m%d-%H%M%S).dump"
	docker compose exec -T db pg_dump -U wormholesystems -d wormholesystems -Fc >"$dump.partial"
	local tables
	tables=$(docker compose exec -T db pg_restore --list <"$dump.partial" | grep -c 'TABLE DATA' || true)
	[ "$tables" -gt 50 ] || die "the dump lists only $tables tables; not going on without one"
	mv "$dump.partial" "$dump"
	echo "$dump ($(du -h "$dump" | cut -f1), $tables tables)"

	say "Stop the old API and renumber our migration"
	docker compose stop api
	if [ "$version" = 41 ]; then
		local updated
		updated=$(psql_db "update _sqlx_migrations set version = 46
		                   where version = 41 and description = '$OLD_MIGRATION'
		                   returning version" | grep -c 46 || true)
		if [ "$updated" != 1 ]; then
			docker compose start api
			die "the renumbering touched $updated rows, not 1; the old API is started again"
		fi
	fi

	DUMP_FOR_ROLLBACK=$dump

	say "Start the new API and web"
	docker compose up -d --wait api web

	say "Check"
	local max
	max=$(psql_db "select max(version) from _sqlx_migrations")
	[ "$max" -ge 48 ] || die "migrations stop at $max, not 48"
	local now_echo
	now_echo=$(psql_db "select count(*) from maps where naming_strategy = 'echo'")
	[ "$now_echo" = "$echo_maps" ] || die "$now_echo maps on the echo strategy, expected $echo_maps"
	local health
	health=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 "$SITE/api/healthz")
	[ "$health" = 200 ] || die "$SITE/api/healthz answers $health"
	echo "migrations at $max, $now_echo echo maps, site healthy, web on node $(docker compose exec -T web node --version)"

	say "Done. Keep $dump until the maps have been checked."
}

rollback() {
	local dump=${1:?"usage: rollback <dump>"}
	[ -f "$dump" ] || die "no dump at $dump"

	say "Put the old images and code back"
	for s in api web; do
		docker tag "wormholesystems-$s:$ROLLBACK_TAG" "ghcr.io/wormholesystems/wormholesystems-$s:main"
	done
	git checkout --quiet "$ROLLBACK_TAG"

	say "Restore $dump"
	docker compose stop api web
	docker compose exec -T db dropdb --force -U wormholesystems wormholesystems
	docker compose exec -T db createdb -U wormholesystems wormholesystems
	docker compose exec -T db pg_restore -U wormholesystems -d wormholesystems --no-owner <"$dump"

	say "Start the old API and web"
	docker compose up -d --wait api web
	curl -s -o /dev/null -w "healthz %{http_code}\n" --max-time 15 "$SITE/api/healthz"
	echo "The checkout is on $ROLLBACK_TAG, detached; 'git switch echo' once the branch is fixed."
}

case "${1:-deploy}" in
deploy) deploy "${2:-}" ;;
rollback) rollback "${2:-}" ;;
*) die "usage: deploy [ref] | rollback <dump>" ;;
esac
