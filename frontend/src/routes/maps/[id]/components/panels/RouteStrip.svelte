<script lang="ts">
	// A route in the spirit of the EVE client's: one mark per system on the way, in the
	// system's class colour, ending on the destination. The name is one hover away. Every
	// square is the same size, so strips in a list line up system for system, and a long
	// route wraps onto another line the way it does in the client.
	//
	// The squares are bare spans and the strip listens for all of them. A list can hold
	// thousands (a hundred rows of thirty-odd jumps), and a tooltip of its own on each made
	// redrawing the list take a second; the one hint here exists only while a square is
	// under the pointer.
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { classMeta } from '$lib/map/classes';
	import { systemResolver } from '$lib/resolve-cache.svelte';
	import { cn } from '$lib/utils';

	let {
		systems,
		quiet = false,
		onactivate,
		class: className,
	}: {
		/** The systems on the way, in order; the last one is the destination. */
		systems: number[];
		/** No hints: the full route is already open next to it, and the two would overlap. */
		quiet?: boolean;
		/** A square was clicked. The gaps and the unused end of the line are not the strip's. */
		onactivate?: () => void;
		class?: string;
	} = $props();

	$effect(() => {
		systemResolver.ensure(systems);
	});

	let hovered = $state<{ square: HTMLElement; index: number } | null>(null);
	const hint = $derived.by(() => {
		if (hovered === null || quiet) return null;
		const system = systemResolver.get(systems[hovered.index]);
		return system ? `${system.name} · ${hovered.index + 1}j` : null;
	});

	function over(event: PointerEvent) {
		const square = (event.target as HTMLElement).closest<HTMLElement>('[data-index]');
		hovered = square ? { square, index: Number(square.dataset.index) } : null;
	}

	function click(event: MouseEvent) {
		if (!onactivate || !(event.target as HTMLElement).closest('[data-index]')) return;
		event.stopPropagation();
		onactivate();
	}

	function colour(id: number): string | undefined {
		const system = systemResolver.get(id);
		if (!system) return undefined;
		const { token } = classMeta(system.wormhole_class_id, system.security);
		return `border-color: var(--color-${token})`;
	}
</script>

<!-- No width of its own: it fills the track the list gives it and wraps there, which is why
     the lists make that track the flexible one. Left to its content it would be one square
     wide. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -- a pointer shortcut; the count beside it is the button, and opens the same route. -->
<span
	class={cn('flex min-w-0 flex-1 flex-wrap gap-0.5', className)}
	data-testid="route-strip"
	onpointerover={over}
	onpointerleave={() => (hovered = null)}
	onpointerdown={() => (hovered = null)}
	onclick={click}
>
	<!-- A two pixel line per system, not a filled square: a list of filled ones is a wall of
	     saturated colour, louder than the names beside it. The cell is still a square to
	     aim at, and the one under the pointer fills in. -->
	{#each systems as id, i (i)}
		<span
			class={cn(
				'h-2 w-2 border-b-2 border-muted-foreground/30 hover:border-b-8',
				onactivate && 'cursor-pointer',
			)}
			style={colour(id)}
			data-index={i}
			data-testid="route-strip-system"
		></span>
	{/each}
</span>

{#if hovered && hint}
	<Tooltip.Root open>
		<Tooltip.Content customAnchor={hovered.square}>{hint}</Tooltip.Content>
	</Tooltip.Root>
{/if}
