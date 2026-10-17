// An alias extends its parent's, so it carries the path: `1` → `11` → `112`, or `A` → `AB`
// → `ABC`. A bookmark therefore says which way home is without reading the map.

import { generateEchoAlias, type EchoContext } from './echo';

export type AliasScheme = 'numeric' | 'alphabetical' | 'echo';

/** K-space exits take a reserved letter, so a wormhole chain never reads as the way out. */
export type AliasTargetKind = 'wormhole' | 'h' | 'l' | 'n' | 'p';

const KSPACE_KINDS: readonly string[] = ['h', 'l', 'n', 'p'];

/** A-Z less H, L, N and P: those are reserved for k-space exits. */
const WORMHOLE_LETTERS = 'ABCDEFGIJKMOQRSTUVWXYZ';

/** Whether `alias` is the map's ignored alias (e.g. HOME). An empty setting matches nothing. */
export function isIgnoredAlias(
	alias: string | null | undefined,
	ignoredAlias: string | null | undefined,
): boolean {
	const ignored = (ignoredAlias ?? '').trim();
	if (!ignored) return false;
	return (alias ?? '').trim().toLowerCase() === ignored.toLowerCase();
}

/** The reserved letter for a target's class, or undefined for wormholes. */
export function aliasTargetKind(
	targetIsWormhole: boolean,
	classShort: string | null | undefined,
): AliasTargetKind | undefined {
	if (targetIsWormhole) return 'wormhole';
	const kind = (classShort ?? '').toLowerCase();
	return KSPACE_KINDS.includes(kind) ? (kind as AliasTargetKind) : undefined;
}

/** The smallest positive integer not in `used`, so a freed alias is reused before growing. */
function lowestFree(used: Set<number>): number {
	let index = 1;
	while (used.has(index)) index++;
	return index;
}

/**
 * The lowest unused letter extending `prefix`. Only aliases one character longer count, so
 * k-space exits (`AH1`) and grandchildren (`ABA`) are not mistaken for direct children.
 */
function nextWormholeLetter(prefix: string, aliases: string[]): string {
	const used = new Set<number>();
	for (const alias of aliases) {
		if (alias.length !== prefix.length + 1 || !alias.startsWith(prefix)) continue;
		const index = WORMHOLE_LETTERS.indexOf(alias.slice(prefix.length));
		if (index !== -1) used.add(index);
	}
	let index = 0;
	while (used.has(index)) index++;
	// Past 22 direct children the sequence repeats rather than blocking the jump.
	return WORMHOLE_LETTERS[Math.min(index, WORMHOLE_LETTERS.length - 1)];
}

/** The lowest unused index for a k-space exit, e.g. `AH1`, `AH2` for highsec exits of `A`. */
function nextKspaceIndex(prefix: string, letter: string, aliases: string[]): number {
	const marker = `${prefix}${letter}`;
	const used = new Set<number>();
	for (const alias of aliases) {
		if (!alias.startsWith(marker)) continue;
		const tail = alias.slice(marker.length);
		// Anchored, so `AH1A` is not counted as an `AH` index.
		if (/^\d+$/.test(tail)) used.add(Number.parseInt(tail, 10));
	}
	return lowestFree(used);
}

/**
 * The next child alias, given the parent's and every alias in use. Numeric children of `1`
 * are `11`, `12`; alphabetical ones extend with a letter, k-space exits with their reserved
 * letter plus an index. Gaps are filled before the sequence grows, and matching is
 * case-insensitive.
 */
export function guessNextAlias(
	parentAlias: string | null | undefined,
	aliases: string[],
	opts?: {
		scheme?: AliasScheme;
		targetKind?: AliasTargetKind;
		ignoredAlias?: string;
		echo?: EchoContext;
	},
): string {
	// Echo runs before the upper-casing below: its type characters are capitals while the
	// branch and slot are lower case, so `a5s` would come back as `A5S` and stop matching
	// the aliases already on the map. It also reads the parent's alias for a branch letter
	// rather than extending it as a prefix.
	if (opts?.scheme === 'echo' && opts.echo) {
		return generateEchoAlias(parentAlias, aliases, opts.echo) ?? '';
	}

	let prefix = (parentAlias ?? '').trim().toUpperCase();
	// The home system is not a chain node, so its children start a fresh sequence.
	if (isIgnoredAlias(prefix, opts?.ignoredAlias)) prefix = '';

	const known = aliases.map((alias) => alias.trim().toUpperCase());

	if (opts?.scheme === 'alphabetical') {
		const kind = opts.targetKind;
		if (kind && kind !== 'wormhole') {
			const letter = kind.toUpperCase();
			return `${prefix}${letter}${nextKspaceIndex(prefix, letter, known)}`;
		}
		return `${prefix}${nextWormholeLetter(prefix, known)}`;
	}

	const numeric = known.filter(
		(alias) =>
			alias.length > prefix.length &&
			alias.startsWith(prefix) &&
			/^\d+$/.test(alias.slice(prefix.length)),
	);
	// Drop grandchildren: `121` extends `12`, which itself extends the prefix.
	const direct = numeric.filter(
		(alias) =>
			!numeric.some(
				(other) => other !== alias && other.length < alias.length && alias.startsWith(other),
			),
	);

	const used = new Set<number>();
	for (const alias of direct) {
		const index = Number.parseInt(alias.slice(prefix.length), 10);
		if (!Number.isNaN(index)) used.add(index);
	}
	return `${prefix}${lowestFree(used)}`;
}

/**
 * An alias for a system reached by a jump, or null when it should not be aliased.
 *
 * A target is worth aliasing when it is itself a wormhole, or when the system jumped from
 * is part of the chain: a k-space exit of an aliased hole continues the chain, but a hop
 * between two unaliased k-space systems is just travel and gets no name.
 */
export function suggestAlias(params: {
	parentAlias: string | null | undefined;
	targetIsWormhole: boolean;
	originIsWormhole: boolean;
	aliases: string[];
	scheme?: AliasScheme;
	targetKind?: AliasTargetKind;
	ignoredAlias?: string;
	echo?: EchoContext;
}): string | null {
	const originIsAliased = Boolean(params.parentAlias && params.parentAlias.trim());
	// Home has no alias of its own to inherit from, so echo names a hole scanned there on
	// the strength of `originIsHome` alone.
	const echoFromHome = params.scheme === 'echo' && Boolean(params.echo?.originIsHome);
	if (!params.targetIsWormhole && !params.originIsWormhole && !originIsAliased && !echoFromHome) {
		return null;
	}

	const alias = guessNextAlias(params.parentAlias, params.aliases, {
		scheme: params.scheme,
		targetKind: params.targetKind,
		ignoredAlias: params.ignoredAlias,
		echo: params.echo,
	});
	// Echo declines to name some destinations (an abyssal class, a branch with no free
	// slot); an empty suggestion is no suggestion.
	return alias === '' ? null : alias;
}
