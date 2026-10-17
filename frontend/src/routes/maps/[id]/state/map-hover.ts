// Row-hover from a side panel: the named system lights up on the map, and its computed
// route (when the row has one) replaces the pinned highlight.

import type { RouteResult } from '$lib/routing/algorithm';
import { solarSystemId } from '$lib/map/system';
import type { MapState } from './map-state.svelte';

export function hoverSystem(
	map: MapState,
	targetSolarSystemId: number | null,
	route?: RouteResult,
): void {
	const placed =
		targetSolarSystemId === null
			? undefined
			: map.systems.visible.find((s) => solarSystemId(s) === targetSolarSystemId);
	map.hoveredSystemId = placed?.id ?? null;
	map.route.hoverPath = route?.route.map((s) => s.id) ?? null;
}

export function clearHover(map: MapState): void {
	map.hoveredSystemId = null;
	map.route.hoverPath = null;
}
