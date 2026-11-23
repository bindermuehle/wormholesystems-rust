// What the navigation pickers offer before anything is typed: the systems already in play.
// Each sits inside its own picker, so choosing one is unambiguous about which field it fills.

import type { CharacterRef } from '$lib/api/types/CharacterRef';
import type { SystemSearchResult } from '$lib/api/types/SystemSearchResult';
import type { MapState } from '../../../state/map-state.svelte';

export interface PickerSuggestion {
	system: SystemSearchResult;
	reason: string;
	icon: 'selected' | 'location' | 'pinned';
}

/** Read inside a `$derived`: it follows the selection, the pilot and both watchlists. */
export function pickerSuggestions(map: MapState): PickerSuggestion[] {
	const picks: { id: number; reason: string; icon: PickerSuggestion['icon'] }[] = [];
	const offer = (id: number, reason: string, icon: PickerSuggestion['icon']) => {
		if (!picks.some((p) => p.id === id)) picks.push({ id, reason, icon });
	};
	const active = map.activeSystem;
	if (active?.kind === 'system') offer(active.solar_system_id, 'Selected system', 'selected');
	for (const [id, names] of pilotLocations(map.characters.mine)) {
		offer(id, whereLabel(names, map.characters.mine.length), 'location');
	}
	for (const entry of map.personalWatchlist.all.filter((w) => w.is_pinned).slice(0, 5)) {
		offer(entry.solar_system_id, 'Pinned on your watchlist', 'pinned');
	}
	for (const entry of map.watchlist.all.filter((w) => w.is_pinned).slice(0, 5)) {
		offer(entry.solar_system_id, 'Pinned on the watchlist', 'pinned');
	}
	// Only those we can render as a proper row; an unresolved one would show as a bare id.
	return picks.flatMap((p) => {
		const system = map.systemInfo(p.id);
		return system ? [{ system, reason: p.reason, icon: p.icon }] : [];
	});
}

/** Every system one of your online pilots is in, with who is there, active pilot first. */
export function pilotLocations(characters: CharacterRef[]): Map<number, string[]> {
	const located = new Map<number, string[]>();
	const activeFirst = [...characters].sort((a, b) => Number(b.is_active) - Number(a.is_active));
	for (const c of activeFirst) {
		if (!c.online || c.solar_system_id === null) continue;
		located.set(c.solar_system_id, [...(located.get(c.solar_system_id) ?? []), c.name]);
	}
	return located;
}

/** "Where you are" for someone with one character; names once there is more than one. */
function whereLabel(names: string[], characterCount: number): string {
	if (characterCount <= 1) return 'Where you are';
	if (names.length === 1) return `Where ${names[0]} is`;
	return `Where ${names.slice(0, -1).join(', ')} and ${names.at(-1)} are`;
}
