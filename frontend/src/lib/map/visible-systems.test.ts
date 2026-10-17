import { describe, expect, it } from 'vitest';
import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import { treeEdges } from './edges';
import type { Vec2 } from './geometry';

// The rule the map state applies: everything is raised, the setting says who sees it.
const drawn = (systems: MapSystemView[], ghosting: boolean) =>
	ghosting ? systems : systems.filter((s) => s.kind !== 'ghost');

const system = (id: number, kind: 'system' | 'ghost' = 'system'): MapSystemView =>
	({ kind, id, alias: null, is_home: false, is_pinned: false }) as unknown as MapSystemView;

const edge = (id: number, from_system: number, to_system: number): MapConnection =>
	({ id, from_system, to_system, type: 'wormhole' }) as unknown as MapConnection;

describe('what the map draws', () => {
	it('leaves ghosts out when the map hides them, and keeps the rest', () => {
		const systems = [system(1), system(2), system(3, 'ghost')];
		expect(drawn(systems, false).map((s) => s.id)).toEqual([1, 2]);
		expect(drawn(systems, true).map((s) => s.id)).toEqual([1, 2, 3]);
	});

	it('drops the edge to a hidden ghost, because it has no position', () => {
		// Positions come from the layout, which is fed the drawn systems. An edge whose
		// endpoint has none is skipped by the router -- no separate filtering needed.
		const positions = new Map<number, Vec2>([
			[1, { x: 0, y: 0 }],
			[2, { x: 100, y: 0 }],
		]);
		const connections = [edge(10, 1, 2), edge(11, 1, 3)];
		const geometry = treeEdges(connections, positions, 40);
		expect(geometry.has(10)).toBe(true);
		expect(geometry.has(11)).toBe(false);
	});

	it('still has the hidden ghost in the full list, which is where its name lives', () => {
		const systems = [system(1), system(3, 'ghost')];
		// The signature panel reads the full list, so a hidden hole still shows its alias.
		expect(systems.some((s) => s.kind === 'ghost')).toBe(true);
		expect(drawn(systems, false).some((s) => s.kind === 'ghost')).toBe(false);
	});
});
