<script lang="ts">
	import NavigationIcon from '@lucide/svelte/icons/navigation';
	import UsersIcon from '@lucide/svelte/icons/users';
	import type { Snippet } from 'svelte';

	import { page } from '$app/state';
	import type { RouteStep } from '$lib/routing/algorithm';
	import { Button } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import * as Popover from '$lib/components/ui/popover';
	import EveImage from '$lib/components/EveImage.svelte';
	import RouteList from './RouteList.svelte';
	import RouteStrip from './RouteStrip.svelte';
	import type { MapState } from '../../state/map-state.svelte';

	let {
		map,
		steps,
		children,
	}: {
		map: MapState;
		steps: RouteStep[];
		children: Snippet;
	} = $props();

	let open = $state(false);
	// Not rendered at all when turned off: a long list is thousands of marks.
	const routePreviews = $derived(page.data.preferences?.route_previews ?? true);
	const jumps = $derived(Math.max(0, steps.length - 1));
	const destinationId = $derived(steps.at(-1)?.id ?? null);
	const online = $derived(map.characters.online);
</script>

<Popover.Root bind:open>
	<!-- Fills its grid track, strip to the left and count to the right, so both line up down
	     the list. Only the count and the squares open the route: the track can be most of the
	     row, and the empty part of it stays the row's own to click. -->
	<span class="flex w-full items-center justify-end gap-2">
		<!-- The route itself, where the card has the room for it: the systems after the one
		     it starts from, so the last square is the row's own. -->
		{#if jumps > 0 && routePreviews}
			<RouteStrip
				systems={steps.slice(1).map((step) => step.id)}
				quiet={open}
				onactivate={() => (open = true)}
				class="hidden @[36rem]/routes:flex"
			/>
		{/if}
		<Popover.Trigger
			class="shrink-0"
			data-testid="jump-badge"
			onclick={(ev: MouseEvent) => ev.stopPropagation()}
		>
			{@render children()}
		</Popover.Trigger>
	</span>
	<!-- Same as the connection popover: without this the auto-focus lands on the first
	     tooltip trigger in the list and pops a tooltip nobody asked for. -->
	<Popover.Content
		class="w-[26rem] gap-0 p-0"
		align="end"
		data-testid="route-popover"
		onOpenAutoFocus={(ev) => ev.preventDefault()}
	>
		<div
			class="flex items-center justify-between gap-2 border-b border-border/50 bg-muted/30 px-3 py-2"
		>
			<span class="font-mono text-[10px] tracking-wider text-muted-foreground uppercase">
				Route · {jumps} jumps
			</span>
			{#if destinationId !== null && online.length > 0}
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="secondary"
								size="sm"
								class="h-6 shrink-0 gap-1 px-2 text-[10px]"
							>
								<NavigationIcon class="size-3" />
								Set Destination
							</Button>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content align="end">
						{#each online as c (c.character_id)}
							<DropdownMenu.Item
								class="text-xs"
								onclick={() => map.waypoints.set(destinationId, c.character_id, true)}
							>
								<EveImage
									kind="character"
									id={c.character_id}
									size={32}
									class="size-5 rounded-lg"
								/>
								{c.name}
							</DropdownMenu.Item>
						{/each}
						{#if online.length > 1}
							<DropdownMenu.Separator />
							<DropdownMenu.Item
								class="text-xs"
								onclick={() => map.waypoints.setAll(destinationId, true)}
							>
								<UsersIcon class="size-4" />
								All Characters
							</DropdownMenu.Item>
						{/if}
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			{/if}
		</div>
		<div class="p-2">
			<RouteList
				steps={map.route.withSignatures(steps)}
				onignore={(id) => map.route.ignoreSystem(id)}
			/>
		</div>
	</Popover.Content>
</Popover.Root>
