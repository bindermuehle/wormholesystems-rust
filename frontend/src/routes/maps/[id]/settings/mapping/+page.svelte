<script lang="ts">
	// What the map does for you while you fly. Location sharing gates the rest, and is shown
	// as gating it rather than leaving dead toggles around.
	//
	// The scanning card is the map's setting, not this viewer's: an unmapped hole put on the
	// map is a node everyone sees.
	import { createQuery } from '@tanstack/svelte-query';
	import { AUTOMATION, automationPatch, settingBlocker, settingOn } from '$lib/map/user-settings';
	import { api } from '$lib/api/client';
	import { apiAction } from '$lib/api/mutations';
	import { key, q } from '$lib/api/queries';
	import SettingRow from '$lib/components/settings/SettingRow.svelte';
	import * as Card from '$lib/components/ui/card';
	import { Switch } from '$lib/components/ui/switch';
	import {
		CORPORATION_STRUCTURES_SCOPE,
		LOCATION_SCOPE,
		NOTIFICATIONS_SCOPE,
	} from '$lib/esi/scopes';
	import { mapSettings } from '../context.svelte';

	const ctx = mapSettings();
	const mapId = $derived(ctx.mapId);
	const scopesQuery = createQuery(() => q.myScopes());
	const scopes = $derived(scopesQuery.data ?? []);
	const settings = $derived(ctx.settings);

	const hasLocation = $derived(scopes.some((s) => s.scope === LOCATION_SCOPE && s.granted));
	const MAPPING = (
		['tracking_allowed', 'prompt_for_signature', 'preselect_signature', 'suggest_alias'] as const
	).map((k) => AUTOMATION[k]);
	const canManage = $derived(ctx.canManage);
	// Sharing reads with whichever of the two permissions a character granted, so it is
	// only held back when there is neither.
	const hasStructureScope = $derived(
		scopes.some(
			(s) => s.granted && [NOTIFICATIONS_SCOPE, CORPORATION_STRUCTURES_SCOPE].includes(s.scope),
		),
	);
	const ghosting = $derived(ctx.view.map.ghost_unlinked_wormholes);

	const saveUserSettings = ctx.saveUserSettings;
	const mapAct = apiAction(() => [key.mapView(mapId)]);

	function updateMap(ghost: boolean) {
		mapAct.mutate(() => api.updateMap({ map_id: mapId, ghost_unlinked_wormholes: ghost }));
	}
</script>

<div class="flex flex-col gap-6">
	<Card.Root>
		<Card.Header>
			<Card.Title>Scanning</Card.Title>
			<Card.Description>What a pasted scan puts on the map, for everyone on it.</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col py-0">
			<SettingRow
				id="ghost-unlinked-wormholes"
				label="Put unmapped wormholes on the map"
				description="A pasted wormhole signature that is not on the map yet gets a node hanging off the system it was scanned in, with no system behind it. Lay the chain out and name it before anyone flies it; assign the system from the node's menu once someone has."
				disabled={!canManage}
				blocked={canManage ? undefined : 'Only a manager can change this.'}
			>
				{#snippet control()}
					<Switch
						checked={ghosting}
						disabled={!canManage}
						aria-label="Put unmapped wormholes on the map"
						onCheckedChange={(v) => updateMap(v)}
					/>
				{/snippet}
			</SettingRow>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Mapping</Card.Title>
			<Card.Description>
				What happens automatically as you fly the chain. Yours alone, per map.
			</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col py-0">
			{#each MAPPING as setting (setting.key)}
				{@const blocked = settingBlocker(settings, setting.key, hasLocation)}
				<SettingRow
					id={setting.key.replaceAll('_', '-')}
					label={setting.label}
					description={setting.body}
					disabled={blocked !== null}
					blocked={blocked ?? undefined}
				>
					{#snippet control()}
						<Switch
							checked={settingOn(settings, setting.key, hasLocation)}
							disabled={blocked !== null}
							aria-label={setting.label}
							onCheckedChange={(v) => saveUserSettings(automationPatch(setting.key, v))}
						/>
					{/snippet}
				</SettingRow>
			{/each}
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Structures</Card.Title>
			<Card.Description>
				What this map may know about your corporations' structures. Yours alone, per map.
			</Card.Description>
		</Card.Header>
		<Card.Content class="flex flex-col py-0">
			<SettingRow
				id="structures-shared"
				label="Share my structures with this map"
				description="Members of this map see attacks on your corporations' structures, towers, customs offices and skyhooks, and how long their fuel lasts, and the map's Discord alerts can announce them. Read with each of your characters that granted the Notifications or Corporation structures permission, and only what that character can see in game. Off by default; turning it off hides everything again."
				blocked={hasStructureScope
					? undefined
					: 'Nothing is read until a character grants Notifications or Corporation structures, under Settings, Characters.'}
			>
				{#snippet control()}
					<Switch
						checked={settings?.structures_shared ?? false}
						aria-label="Share my structures with this map"
						onCheckedChange={(v) => saveUserSettings({ structures_shared: v })}
						data-testid="setting-structures-shared"
					/>
				{/snippet}
			</SettingRow>
		</Card.Content>
	</Card.Root>
</div>
