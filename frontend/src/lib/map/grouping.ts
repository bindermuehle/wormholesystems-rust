// The per-system rollups behind the node badges: signatures and pilots keyed by solar
// system, connection counts keyed by placement.

import type { MapCharacter } from '$lib/api/types/MapCharacter';
import type { MapConnection } from '$lib/api/types/MapConnection';
import type { Signature } from '$lib/api/types/Signature';

export interface SigCounts {
	total: number;
	uncategorized: number;
	wormholes: number;
}

export function sigCountsBySystem(sigs: Signature[]): Map<number, SigCounts> {
	const out = new Map<number, SigCounts>();
	for (const s of sigs) {
		const c = out.get(s.solar_system_id) ?? { total: 0, uncategorized: 0, wormholes: 0 };
		c.total += 1;
		if (s.group === 'unknown') c.uncategorized += 1;
		if (s.group === 'wormhole') c.wormholes += 1;
		out.set(s.solar_system_id, c);
	}
	return out;
}

export function pilotsBySystem(characters: MapCharacter[]): Map<number, MapCharacter[]> {
	const out = new Map<number, MapCharacter[]>();
	for (const c of characters) {
		if (c.solar_system_id === null) continue;
		const list = out.get(c.solar_system_id) ?? [];
		list.push(c);
		out.set(c.solar_system_id, list);
	}
	return out;
}

/** Both endpoints count: the badge says how many lines touch the node. */
/**
 * How many of a placement's edges lead to a hole nobody has been through, by placement.
 *
 * A scanned wormhole is drawn its own node from the moment it is found, so counting the
 * scans that have no connection -- which is what "unmapped" used to mean -- now counts
 * nothing at all. The question a node's badge answers is unchanged: how many holes here
 * has nobody flown? That is the edges whose far side is still a ghost.
 */
export function unmappedCountByPlacement(
	connections: MapConnection[],
	isGhost: (placementId: number) => boolean,
): Map<number, number> {
	const out = new Map<number, number>();
	const bump = (id: number) => out.set(id, (out.get(id) ?? 0) + 1);
	for (const c of connections) {
		if (isGhost(c.to_system)) bump(c.from_system);
		if (isGhost(c.from_system)) bump(c.to_system);
	}
	return out;
}

export function connectionCountByPlacement(connections: MapConnection[]): Map<number, number> {
	const out = new Map<number, number>();
	for (const c of connections) {
		out.set(c.from_system, (out.get(c.from_system) ?? 0) + 1);
		out.set(c.to_system, (out.get(c.to_system) ?? 0) + 1);
	}
	return out;
}
