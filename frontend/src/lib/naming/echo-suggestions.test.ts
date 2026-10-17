import { describe, expect, it } from 'vitest';
import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import type { Signature } from '$lib/api/types/Signature';
import type { SignatureCatalog } from '$lib/api/types/SignatureCatalog';
import { echoSuggestions } from './echo-suggestions';

const system = (
	id: number,
	over: { alias?: string | null; home?: boolean; statics?: string[]; classId?: number } = {},
): MapSystemView =>
	({
		kind: 'system',
		id,
		alias: over.alias ?? null,
		is_home: over.home ?? false,
		wormhole_class_id: over.classId ?? 5,
		statics: (over.statics ?? []).map((code) => ({ code, dest_class: null })),
	}) as unknown as MapSystemView;

const edge = (id: number, from_system: number, to_system: number): MapConnection =>
	({ id, from_system, to_system }) as unknown as MapConnection;

// Two catalog entries: a C5 hole and a null-sec one.
const catalog = {
	categories: [],
	types: [
		{ id: 1, signature: 'H296', name: 'H296', target_class: 5 },
		{ id: 2, signature: 'K162', name: 'K162', target_class: 9 },
	],
} as unknown as SignatureCatalog;

const sig = (
	id: number,
	over: { type?: number | null; conn?: number | null; at?: string } = {},
): Signature =>
	({
		id,
		group: 'wormhole',
		signature_type_id: over.type ?? null,
		connection_id: over.conn ?? null,
		solar_system_id: 100,
		created_at: over.at ?? '2026-01-01T00:00:00Z',
	}) as unknown as Signature;

describe('echoSuggestions', () => {
	it('gives two unscanned holes of the same class different slots', () => {
		const home = system(1, { home: true, statics: [] });
		const out = echoSuggestions({
			systems: [home, system(2, { alias: 'a5a' })],
			connections: [],
			system: home,
			sigs: [
				sig(10, { type: 1, at: '2026-01-01T01:00:00Z' }),
				sig(11, { type: 1, at: '2026-01-01T02:00:00Z' }),
			],
			catalog,
		});
		const names = [out.get(10)?.alias, out.get(11)?.alias];
		expect(new Set(names).size).toBe(2);
		expect(names).toEqual(['a5b', 'a5c']);
	});

	it("gives home's static the s slot and the next hole its own branch", () => {
		const home = system(1, { home: true, statics: ['H296'] });
		const out = echoSuggestions({
			systems: [home],
			connections: [],
			system: home,
			sigs: [
				sig(10, { type: 1, at: '2026-01-01T01:00:00Z' }),
				sig(11, { type: 1, at: '2026-01-01T02:00:00Z' }),
			],
			catalog,
		});
		expect(out.get(10)?.alias).toBe('a5s');
		// The second H296 is not the static, so it starts past the reserved branch.
		expect(out.get(11)?.alias).toBe('b5a');
	});

	it('reports the name a linked hole already leads to, as settled', () => {
		const home = system(1, { home: true });
		const far = system(2, { alias: 'a5s' });
		const out = echoSuggestions({
			systems: [home, far],
			connections: [edge(50, 1, 2)],
			system: home,
			sigs: [sig(10, { type: 1, conn: 50 })],
			catalog,
		});
		expect(out.get(10)).toEqual({ alias: 'a5s', homeward: false, settled: true });
	});

	it('marks the hole that leads back toward home', () => {
		// home(1) -- a5s(2) -- b4a(3); standing in 2, the hole to 1 is the way back.
		const home = system(1, { home: true, alias: 'HOME' });
		const here = system(2, { alias: 'a5s' });
		const out = echoSuggestions({
			systems: [home, here, system(3, { alias: 'a4a' })],
			connections: [edge(50, 1, 2), edge(51, 2, 3)],
			system: here,
			sigs: [sig(10, { type: 1, conn: 50 }), sig(11, { type: 1, conn: 51 })],
			catalog,
		});
		expect(out.get(10)?.homeward).toBe(true);
		expect(out.get(11)?.homeward).toBe(false);
	});

	it('suggests a name for a hole drawn as a ghost, which is linked but unnamed', () => {
		// With `ghost_unlinked_wormholes` on, a scanned hole is linked to a nameless ghost
		// from the moment it appears. That row needs a suggestion more than any other.
		const home = system(1, { home: true, statics: ['H296'] });
		const ghost = { kind: 'ghost', id: 2, alias: null, is_home: false } as unknown as MapSystemView;
		const out = echoSuggestions({
			systems: [home, ghost],
			connections: [edge(50, 1, 2)],
			system: home,
			sigs: [sig(10, { type: 1, conn: 50 })],
			catalog,
		});
		expect(out.get(10)).toEqual({ alias: 'a5s', homeward: false, settled: false });
	});

	it('takes the class from the far system rather than the type, once it is known', () => {
		// The type only promises a class; a system on the far side is the fact.
		const home = system(1, { home: true });
		const far = system(2, { alias: null, classId: 3 });
		const out = echoSuggestions({
			systems: [home, far],
			connections: [edge(50, 1, 2)],
			system: home,
			// The type says C5, the system says C3.
			sigs: [sig(10, { type: 1, conn: 50 })],
			catalog,
		});
		expect(out.get(10)?.alias).toBe('a3a');
	});

	it('names nothing for a hole whose type nobody has picked', () => {
		const home = system(1, { home: true });
		const out = echoSuggestions({
			systems: [home],
			connections: [],
			system: home,
			sigs: [sig(10, { type: null })],
			catalog,
		});
		expect(out.has(10)).toBe(false);
	});

	it('hands out the same slots whichever order the rows arrive in', () => {
		const home = system(1, { home: true, statics: ['H296'] });
		const first = sig(10, { type: 1, at: '2026-01-01T01:00:00Z' });
		const second = sig(11, { type: 1, at: '2026-01-01T02:00:00Z' });
		const run = (sigs: Signature[]) =>
			echoSuggestions({ systems: [home], connections: [], system: home, sigs, catalog });
		const forward = run([first, second]);
		const reversed = run([second, first]);
		expect(forward.get(10)?.alias).toBe(reversed.get(10)?.alias);
		expect(forward.get(11)?.alias).toBe(reversed.get(11)?.alias);
	});

	it('leaves anything that is not a wormhole alone', () => {
		const home = system(1, { home: true });
		const data = { ...sig(10, { type: 1 }), group: 'data' } as unknown as Signature;
		const out = echoSuggestions({
			systems: [home],
			connections: [],
			system: home,
			sigs: [data],
			catalog,
		});
		expect(out.size).toBe(0);
	});
});
