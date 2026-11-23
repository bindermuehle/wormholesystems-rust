<script lang="ts">
	// The system menu for someone who never thought to right-click: three dots at the end of
	// a row that open the very menu the row's `SystemMenu` already carries. It asks for that
	// menu rather than holding one of its own, so a list of a hundred rows mounts no second
	// menu per row, and the two can never offer different things. Only works inside a
	// `SystemMenu`.
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';

	import { cn } from '$lib/utils';

	let { name, class: className }: { name: string; class?: string } = $props();

	function open(event: MouseEvent) {
		// The row may have a click of its own (expand, select); this is not that.
		event.stopPropagation();
		const button = event.currentTarget as HTMLElement;
		const box = button.getBoundingClientRect();
		button.dispatchEvent(
			new MouseEvent('contextmenu', {
				bubbles: true,
				cancelable: true,
				clientX: box.left,
				clientY: box.bottom,
			}),
		);
	}
</script>

<button
	type="button"
	class={cn('flex items-center text-muted-foreground hover:text-foreground', className)}
	aria-label="Actions for {name}"
	aria-haspopup="menu"
	data-testid="system-menu-button"
	onclick={open}
>
	<EllipsisVerticalIcon class="size-3.5" />
</button>
