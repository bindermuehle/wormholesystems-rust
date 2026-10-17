<script lang="ts">
	import { Command as CommandPrimitive } from "bits-ui";
	import * as InputGroup from "$lib/components/ui/input-group/index.js";
	import SearchIcon from '@lucide/svelte/icons/search';
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(""),
		...restProps
	}: CommandPrimitive.InputProps = $props();
</script>

<div data-slot="command-input-wrapper" class="p-1 pb-0">
	<InputGroup.Root class="h-8! bg-input/20 dark:bg-input/30">
		<!-- A search box is not a login field. bits-ui already says so the standard way
		     (`type=text`, `autocomplete=off`, `aria-autocomplete=list`), but password
		     managers largely ignore that and offer to fill anything they can focus, so
		     each one gets the opt-out it does honour. Harmless where the extension is not
		     installed.

		     Apple's Passwords ignores the data attributes too; it goes by field type, and
		     it does not offer to fill a search field. bits-ui hardcodes `type=text` and
		     wins any merge, so the override has to come after the spread below. The two
		     arbitrary variants drop the native clear button that `type=search` brings. -->
		<CommandPrimitive.Input
			{value}
			data-slot="command-input"
			data-1p-ignore
			data-lpignore="true"
			data-bwignore
			data-form-type="other"
			class={cn(
				"w-full text-xs/relaxed outline-hidden disabled:cursor-not-allowed disabled:opacity-50",
				className
			)}
			{...restProps}
		>
			{#snippet child({ props })}
				<InputGroup.Input
					{...props}
					type="search"
					class={cn(
						props.class as string | undefined,
						"[&::-webkit-search-cancel-button]:appearance-none [&::-webkit-search-decoration]:appearance-none"
					)}
					bind:value
					bind:ref
				/>
			{/snippet}
		</CommandPrimitive.Input>
		<InputGroup.Addon>
			<SearchIcon class="size-3.5 shrink-0 opacity-50" />
		</InputGroup.Addon>
	</InputGroup.Root>
</div>
