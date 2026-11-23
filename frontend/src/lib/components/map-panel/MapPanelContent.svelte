<script lang="ts">
	import type { Snippet } from 'svelte';

	import { page } from '$app/state';
	import { cn } from '$lib/utils';

	let { children }: { children: Snippet } = $props();

	// The lists inside ask this container how wide the card is before they draw a route next
	// to a jump count and hand that column the spare width (`@[36rem]/routes:`). With the
	// preference off there is no such container to ask, so every one of those rules is
	// simply never true and the lists keep their plain layout.
	const routePreviews = $derived(page.data.preferences?.route_previews ?? true);
</script>

<div class={cn('flex-1 overflow-x-hidden overflow-y-auto', routePreviews && '@container/routes')}>
	{@render children()}
</div>
