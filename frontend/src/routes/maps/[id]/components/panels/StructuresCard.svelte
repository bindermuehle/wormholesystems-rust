<script lang="ts">
	// What members share of their corporations' structures: who is shooting what, and what
	// runs dry next. Nothing is here unless somebody turned sharing on for this map, which
	// is theirs to do and to undo, from the header.
	import CheckCircleIcon from '@lucide/svelte/icons/check-circle-2';
	import CheckIcon from '@lucide/svelte/icons/circle-check';
	import FuelIcon from '@lucide/svelte/icons/fuel';
	import SearchIcon from '@lucide/svelte/icons/search';
	import ShareIcon from '@lucide/svelte/icons/share-2';
	import SwordsIcon from '@lucide/svelte/icons/swords';

	import { createQuery } from '@tanstack/svelte-query';

	import { q } from '$lib/api/queries';
	import type { MapStructure } from '$lib/api/types/MapStructure';
	import type { SystemSearchResult } from '$lib/api/types/SystemSearchResult';
	import ClassBadge from '$lib/components/ClassBadge.svelte';
	import EveImage from '$lib/components/EveImage.svelte';
	import Hint from '$lib/components/Hint.svelte';
	import MapPanel from '$lib/components/map-panel/MapPanel.svelte';
	import MapPanelContent from '$lib/components/map-panel/MapPanelContent.svelte';
	import MapPanelHeader from '$lib/components/map-panel/MapPanelHeader.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Popover from '$lib/components/ui/popover';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { Switch } from '$lib/components/ui/switch';
	import { CORPORATION_STRUCTURES_SCOPE, NOTIFICATIONS_SCOPE, scopeMeta } from '$lib/esi/scopes';
	import { timeAgo, timeAgoShort, utcShort } from '$lib/format';
	import {
		EVENT_TONE_CLASS,
		FUEL_TONE_CLASS,
		aggressorLabel,
		fuelLeftMs,
		fuelTone,
		eventMeta,
		matchesQuery,
		type EventTone,
		stateNote,
		structureLabel,
		timerLeftMs,
	} from '$lib/map/structures';
	import { solarSystemId, toSearchResult } from '$lib/map/system';
	import { ticking } from '$lib/now.svelte';
	import { jumpTone } from '$lib/routing/algorithm';
	import { formatDuration } from '$lib/skyhooks/timer';
	import { cn } from '$lib/utils';
	import { clearHover, hoverSystem } from '../../state/map-hover';
	import type { MapState } from '../../state/map-state.svelte';
	import { routeBatch } from '../../state/route-batch.svelte';
	import SystemMenu from '../system-menu/SystemMenu.svelte';
	import SystemMenuButton from '../system-menu/SystemMenuButton.svelte';
	import RouteOriginBadge from './RouteOriginBadge.svelte';
	import RoutePopover from './RoutePopover.svelte';

	let { map }: { map: MapState } = $props();

	const clock = ticking(60_000);
	const now = $derived(clock.current);

	const data = $derived(map.queries.structures.data ?? null);
	// What the header's filters and the search leave of the two lists.
	let shown = $state<EventTone[]>(['attack', 'fuel', 'routine']);
	let query = $state('');
	const events = $derived(
		(data?.events ?? []).filter(
			(e) =>
				shown.includes(eventMeta(e.kind).tone) &&
				matchesQuery(e, query, aliasOf(e.solar_system_id)),
		),
	);
	// The fuel list goes with the fuel events: one switch for everything about fuel.
	const structures = $derived(
		shown.includes('fuel')
			? (data?.structures ?? []).filter((s) => matchesQuery(s, query, aliasOf(s.solar_system_id)))
			: [],
	);
	const anything = $derived((data?.events.length ?? 0) > 0 || (data?.structures.length ?? 0) > 0);

	/** How old the structure list is: its state and timers are only as fresh as this. */
	const listRead = $derived.by(() => {
		const oldest = (data?.structures ?? []).map((s) => s.updated_at).sort()[0];
		return oldest ? timeAgo(oldest, now) : null;
	});

	const FILTERS: { key: EventTone; label: string; icon: typeof SwordsIcon }[] = [
		{ key: 'attack', label: 'attacks', icon: SwordsIcon },
		{ key: 'fuel', label: 'fuel', icon: FuelIcon },
		{ key: 'routine', label: 'routine changes', icon: CheckIcon },
	];

	/** The map's own name for the system, when it is on the map and has one. */
	function aliasOf(id: number | undefined): string | null {
		if (id === undefined) return null;
		return map.systems.all.find((s) => solarSystemId(s) === id)?.alias ?? null;
	}
	const sharing = $derived(map.userSettings?.structures_shared ?? false);

	const DAY_MS = 24 * 60 * 60 * 1000;
	/** Attacks in the last day: the number worth a glance at the header. */
	const recentAttacks = $derived(
		(data?.events ?? []).filter(
			(e) =>
				eventMeta(e.kind).tone === 'attack' &&
				now.getTime() - new Date(e.occurred_at).getTime() < DAY_MS,
		).length,
	);

	// The two permissions sharing reads with, for whoever is deciding whether to turn it on.
	const scopesQuery = createQuery(() => ({ ...q.myScopes(), enabled: map.signedIn }));
	const PERMISSIONS = [NOTIFICATIONS_SCOPE, CORPORATION_STRUCTURES_SCOPE].map((scope) => ({
		scope,
		name: scopeMeta(scope)?.name ?? scope,
	}));
	const granted = $derived(
		new Set((scopesQuery.data ?? []).filter((s) => s.granted).map((s) => s.scope)),
	);

	function hover(solarSystemId: number | undefined, on: boolean) {
		if (on && solarSystemId !== undefined) hoverSystem(map, solarSystemId);
		else clearHover(map);
	}

	/** What both kinds of row say about where they are. */
	type Placed = Pick<
		MapStructure,
		| 'solar_system_id'
		| 'system_name'
		| 'security'
		| 'region'
		| 'region_id'
		| 'constellation_id'
		| 'wormhole_class_id'
	>;

	const ROW =
		'col-span-full grid grid-cols-subgrid items-center border-b border-border/30 px-3 py-1 text-xs last:border-b-0 hover:bg-muted/30';

	// One search from the origin covers every row, as on the other cards.
	// svelte-ignore state_referenced_locally -- the map instance is stable for this mount.
	const batch = routeBatch(map, () =>
		[...events, ...structures].flatMap((row) => row.solar_system_id ?? []),
	);
	const routes = $derived(batch.routes);

	/** The row's system in the shape the context menu wants, when the row names one. */
	function systemOf(row: Placed): SystemSearchResult | null {
		if (row.solar_system_id === undefined || !row.system_name) return null;
		return toSearchResult({
			id: row.solar_system_id,
			name: row.system_name,
			security: row.security ?? 0,
			region: row.region ?? '',
			region_id: row.region_id ?? 0,
			constellation_id: row.constellation_id ?? 0,
			wormhole_class_id: row.wormhole_class_id ?? null,
		});
	}
</script>

<!-- The column titles over one of the two lists. -->
{#snippet heading(title: string, last: string, read: string | null)}
	<div
		class="col-span-full grid grid-cols-subgrid items-center px-3 pt-2 pb-1 text-[10px] tracking-wider text-muted-foreground uppercase first:pt-0"
	>
		<span></span>
		<span>
			{title}
			{#if read}
				<span
					class="tracking-normal normal-case text-muted-foreground/60"
					data-testid="structures-read"
				>
					· as EVE listed them {read}
				</span>
			{/if}
		</span>
		<span>System</span>
		<span>Region</span>
		<span class="text-right">J</span>
		<span class="text-right">{last}</span>
		<span></span>
	</div>
{/snippet}

{#snippet icon(typeId: number | undefined)}
	{#if typeId}
		<!-- A box of its own: an image that fails to load collapses, and would take the
		     column with it. -->
		<span class="flex size-6 shrink-0">
			<EveImage kind="type" id={typeId} class="size-6 rounded" />
		</span>
	{:else}
		<span class="size-6 shrink-0"></span>
	{/if}
{/snippet}

<!-- Where it is, and how far: class and system, region, jumps from the route origin. -->
{#snippet place(row: Placed)}
	{@const alias = aliasOf(row.solar_system_id)}
	{@const route = row.solar_system_id === undefined ? undefined : routes.get(row.solar_system_id)}
	<span class="flex min-w-0 items-center gap-1.5">
		{#if row.system_name}
			<ClassBadge
				classId={row.wormhole_class_id ?? null}
				security={row.security ?? 0}
				class="shrink-0 text-[10px]"
			/>
			<Hint
				text={alias ? `${alias} · ${row.system_name}` : row.system_name}
				clippedOnly
				focusable={false}
			>
				{#snippet child({ props })}
					<span class="truncate" {...props} data-testid="structure-system">
						{#if alias}<span class="font-medium">{alias}</span> ·
						{/if}{row.system_name}
					</span>
				{/snippet}
			</Hint>
		{/if}
	</span>
	<Hint text={row.region} clippedOnly focusable={false}>
		{#snippet child({ props })}
			<span class="min-w-0 truncate font-mono text-[10px] text-muted-foreground" {...props}>
				{row.region ?? ''}
			</span>
		{/snippet}
	</Hint>
	<span class="flex min-w-8 justify-end">
		{#if route}
			<RoutePopover {map} steps={route.route}>
				<span
					class={cn('cursor-pointer font-medium tabular-nums', jumpTone(route.jumps))}
					data-testid="structure-jumps">{route.jumps}j</span
				>
			</RoutePopover>
		{:else}
			<span class="text-[10px] whitespace-nowrap text-muted-foreground/60">--</span>
		{/if}
	</span>
{/snippet}

{#snippet menu(row: Placed)}
	{#if systemOf(row)}
		<SystemMenuButton name={row.system_name ?? ''} />
	{:else}
		<span></span>
	{/if}
{/snippet}

<!-- A row whose system we know answers a right-click with that system's menu. -->
{#snippet wrapped(row: Placed, line: import('svelte').Snippet)}
	{@const system = systemOf(row)}
	{#if system}
		<SystemMenu {system}>{@render line()}</SystemMenu>
	{:else}
		{@render line()}
	{/if}
{/snippet}

<MapPanel testid="structures-card">
	<MapPanelHeader>
		<span class="inline-flex items-center gap-2">
			Structures
			{#if recentAttacks > 0}
				<span class="font-mono text-red-400" data-testid="structures-attacks">{recentAttacks}</span>
			{/if}
		</span>
		{#snippet actions()}
			{#if map.canWrite && anything}
				<label class="relative flex items-center">
					<SearchIcon class="pointer-events-none absolute left-1.5 size-3 text-muted-foreground" />
					<input
						type="text"
						bind:value={query}
						placeholder="Search"
						aria-label="Search structures"
						class="h-6 w-24 border border-border/60 bg-transparent pr-1.5 pl-5 font-sans text-[11px] normal-case tracking-normal outline-none placeholder:text-muted-foreground/60 focus:border-ring @[36rem]/panel-header:w-36"
						data-testid="structures-search"
					/>
				</label>
				<div class="flex items-center gap-0.5" data-testid="structures-filters">
					{#each FILTERS as filter (filter.key)}
						{@const on = shown.includes(filter.key)}
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon"
										class="size-6"
										aria-pressed={on}
										aria-label="{on ? 'Hide' : 'Show'} {filter.label}"
										data-testid="structures-filter-{filter.key}"
										onclick={() => {
											shown = on ? shown.filter((k) => k !== filter.key) : [...shown, filter.key];
										}}
									>
										<filter.icon
											class={cn(
												'size-3',
												on ? EVENT_TONE_CLASS[filter.key] : 'text-muted-foreground/40',
											)}
										/>
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content>{on ? 'Hide' : 'Show'} {filter.label}</Tooltip.Content>
						</Tooltip.Root>
					{/each}
				</div>
			{/if}
			<RouteOriginBadge {map} />
			{#if map.canWrite}
				<Popover.Root>
					<Popover.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon"
								class={cn('size-6', sharing ? 'text-emerald-400' : 'text-muted-foreground/50')}
								aria-label="Structure sharing"
								data-testid="structures-sharing"
							>
								<ShareIcon />
							</Button>
						{/snippet}
					</Popover.Trigger>
					<Popover.Content class="w-80" align="end">
						<div class="flex flex-col gap-3">
							<label class="flex items-start justify-between gap-3" for="structures-shared">
								<span class="flex flex-col gap-0.5">
									<span class="text-xs font-medium">Share my structures with this map</span>
									<span class="text-[11px] text-muted-foreground">
										Members of this map see attacks on your corporations' structures and how long
										their fuel lasts, and the map's Discord alerts can announce them. Off by
										default, per map.
									</span>
								</span>
								<Switch
									id="structures-shared"
									checked={sharing}
									onCheckedChange={(v) => map.patchUserSettings({ structures_shared: v })}
									data-testid="setting-structures_shared"
								/>
							</label>
							<div class="flex flex-col gap-1.5 border-t pt-3">
								<span class="text-[11px] text-muted-foreground">
									Read with each of your characters that granted these:
								</span>
								{#each PERMISSIONS as permission (permission.scope)}
									<span class="flex items-center justify-between gap-3 text-xs">
										{permission.name}
										{#if granted.has(permission.scope)}
											<span class="flex items-center gap-1 text-[11px] text-emerald-500">
												<CheckCircleIcon class="size-3" />
												Granted
											</span>
										{:else}
											<a
												href="/settings/characters"
												class="text-[11px] text-amber-500 underline"
												data-testid="structures-grant"
											>
												Grant
											</a>
										{/if}
									</span>
								{/each}
							</div>
						</div>
					</Popover.Content>
				</Popover.Root>
			{/if}
		{/snippet}
	</MapPanelHeader>

	<MapPanelContent>
		{#if !map.canWrite}
			<p class="px-3 py-4 text-xs text-muted-foreground" data-testid="structures-members-only">
				Shared structures are for members of this map.
			</p>
		{:else if data !== null && data.sharers === 0}
			<p class="px-3 py-4 text-xs text-muted-foreground" data-testid="structures-nobody">
				Nobody shares their structures with this map yet. Turn it on from the button above to see
				attacks and fuel here.
			</p>
		{:else if anything && events.length === 0 && structures.length === 0}
			<p class="px-3 py-4 text-xs text-muted-foreground" data-testid="structures-no-match">
				Nothing matches the search and filters.
			</p>
		{:else if !anything}
			<p class="px-3 py-4 text-xs text-muted-foreground" data-testid="structures-empty">
				Nothing has happened to the shared structures this week. Fuel timers show once a sharing
				character with the Station Manager role has granted Corporation structures.
			</p>
		{:else}
			<!-- One grid for both lists, every row a subgrid, so the columns line up down the
			     whole card the way they do on the other cards. -->
			<div
				class="mt-1 grid grid-cols-[min-content_minmax(0,1.6fr)_minmax(0,1fr)_minmax(0,1fr)_auto_auto_min-content] gap-x-2 @[36rem]/routes:gap-x-4"
			>
				{#if events.length > 0}
					{@render heading('Recent', 'Ago', null)}
					{#each events as event (event.id)}
						{@const attacker = aggressorLabel(event)}
						{@const meta = eventMeta(event.kind)}
						{@const timer = timerLeftMs(event.timer_ends_at, now)}
						{#snippet line()}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class={ROW}
								data-testid="structure-event"
								data-kind={event.kind}
								onmouseenter={() => hover(event.solar_system_id, true)}
								onmouseleave={() => hover(event.solar_system_id, false)}
							>
								{@render icon(event.type_id)}
								<Hint
									text={[
										`${structureLabel(event)} ${meta.verb}`,
										event.type_name,
										event.corporation_name,
										attacker ? `by ${attacker}` : null,
										event.timer_ends_at ? `timer ends ${utcShort(event.timer_ends_at)} UTC` : null,
									]
										.filter(Boolean)
										.join(' · ')}
									focusable={false}
								>
									{#snippet child({ props })}
										<span class="flex min-w-0 flex-col" {...props}>
											<span class="truncate">{structureLabel(event)}</span>
											<span class={cn('truncate text-[10px]', EVENT_TONE_CLASS[meta.tone])}>
												{meta.verb}{attacker ? ` by ${attacker}` : ''}
												{#if timer !== null}
													<span class="text-foreground" data-testid="structure-timer">
														· {formatDuration(timer)} left
													</span>
												{/if}
											</span>
										</span>
									{/snippet}
								</Hint>
								{@render place(event)}
								<span
									class="w-14 shrink-0 text-right font-mono text-[10px] text-muted-foreground tabular-nums"
								>
									{timeAgoShort(event.occurred_at, now)}
								</span>
								{@render menu(event)}
							</div>
						{/snippet}
						{@render wrapped(event, line)}
					{/each}
				{/if}

				{#if structures.length > 0}
					{@render heading('Fuel', 'Fuel left', listRead)}
					{#each structures as structure (structure.structure_id)}
						{@const tone = fuelTone(structure, now)}
						{@const left = fuelLeftMs(structure, now)}
						{@const note = stateNote(structure)}
						{@const stateLeft = timerLeftMs(note?.until, now)}
						{#snippet line()}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class={ROW}
								data-testid="structure-fuel"
								data-tone={tone}
								onmouseenter={() => hover(structure.solar_system_id, true)}
								onmouseleave={() => hover(structure.solar_system_id, false)}
							>
								{@render icon(structure.type_id)}
								<Hint
									text={[
										structureLabel(structure),
										structure.type_name,
										structure.corporation_name,
										note?.until ? `${note.label} ends ${utcShort(note.until)} UTC` : null,
									]
										.filter(Boolean)
										.join(' · ')}
									focusable={false}
								>
									{#snippet child({ props })}
										<span class="flex min-w-0 flex-col" {...props}>
											<span class="truncate">{structureLabel(structure)}</span>
											<span class="truncate text-[10px] text-muted-foreground">
												{#if note}
													<span class="text-red-400" data-testid="structure-state">
														{note.label}{#if stateLeft !== null}, {formatDuration(stateLeft)} left{:else if note.until},
															timer passed{/if}
													</span>
													·
												{/if}
												{structure.type_name ?? ''}
											</span>
										</span>
									{/snippet}
								</Hint>
								{@render place(structure)}
								<Hint
									text={structure.fuel_expires
										? `Runs out ${utcShort(structure.fuel_expires)} UTC`
										: tone === 'unfueled'
											? 'Not fuelled yet'
											: 'No fuel timer: out of fuel'}
									focusable={false}
								>
									{#snippet child({ props })}
										<span
											class={cn(
												'w-14 shrink-0 text-right font-mono text-[10px] font-semibold tabular-nums',
												FUEL_TONE_CLASS[tone],
											)}
											{...props}
										>
											{left !== null ? formatDuration(left) : tone === 'unfueled' ? '--' : 'out'}
										</span>
									{/snippet}
								</Hint>
								{@render menu(structure)}
							</div>
						{/snippet}
						{@render wrapped(structure, line)}
					{/each}
				{/if}
			</div>
		{/if}
	</MapPanelContent>
</MapPanel>
