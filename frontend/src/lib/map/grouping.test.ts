import { describe, expect, it } from 'vitest';

import type { MapCharacter } from '$lib/api/types/MapCharacter';
import type { MapConnection } from '$lib/api/types/MapConnection';
import type { Signature } from '$lib/api/types/Signature';
import {
	connectionCountByPlacement,
	pilotsBySystem,
	sigCountsBySystem,
	unmappedCountByPlacement,
} from './grouping';

const sig = (solar_system_id: number, group: string): Signature =>
	({ solar_system_id, group }) as Signature;

const pilot = (id: number, solar_system_id: number | null): MapCharacter =>
	({ id, solar_system_id }) as unknown as MapCharacter;

const connection = (id: number, from: number, to: number): MapConnection =>
	({ id, from_system: from, to_system: to }) as MapConnection;

describe('sigCountsBySystem', () => {
	it('counts totals and the unknown/wormhole subsets per system', () => {
		const counts = sigCountsBySystem([
			sig(1, 'wormhole'),
			sig(1, 'unknown'),
			sig(1, 'data'),
			sig(2, 'unknown'),
		]);
		expect(counts.get(1)).toEqual({ total: 3, uncategorized: 1, wormholes: 1 });
		expect(counts.get(2)).toEqual({ total: 1, uncategorized: 1, wormholes: 0 });
		expect(counts.get(3)).toBeUndefined();
	});
});

describe('pilotsBySystem', () => {
	it('groups pilots by where they are and drops the locationless', () => {
		const a = pilot(1, 100);
		const b = pilot(2, 100);
		const lost = pilot(3, null);
		const grouped = pilotsBySystem([a, b, lost]);
		expect(grouped.get(100)).toEqual([a, b]);
		expect(grouped.size).toBe(1);
	});
});

describe('connectionCountByPlacement', () => {
	it('counts both endpoints of every connection', () => {
		const counts = connectionCountByPlacement([
			connection(10, 1, 2),
			connection(11, 1, 3),
			connection(12, 2, 1),
		]);
		expect(counts.get(1)).toBe(3);
		expect(counts.get(2)).toBe(2);
		expect(counts.get(3)).toBe(1);
	});
});

describe('unmappedCountByPlacement', () => {
	const edge = (id: number, from_system: number, to_system: number) =>
		({ id, from_system, to_system }) as unknown as MapConnection;

	it('counts the holes leading to a node nobody has been through', () => {
		// 1 --- 2 (a real system), 1 --- 3 and 1 --- 4 (ghosts).
		const ghosts = new Set([3, 4]);
		const counts = unmappedCountByPlacement(
			[edge(10, 1, 2), edge(11, 1, 3), edge(12, 4, 1)],
			(id) => ghosts.has(id),
		);
		expect(counts.get(1)).toBe(2);
		expect(counts.get(2)).toBeUndefined();
	});

	it('counts from either end of the edge', () => {
		const counts = unmappedCountByPlacement([edge(10, 9, 1)], (id) => id === 9);
		expect(counts.get(1)).toBe(1);
	});

	it('counts nothing on a chain where every hole has been flown', () => {
		const counts = unmappedCountByPlacement([edge(10, 1, 2), edge(11, 2, 3)], () => false);
		expect(counts.size).toBe(0);
	});
});
