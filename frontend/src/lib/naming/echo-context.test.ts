import { describe, expect, it } from 'vitest';
import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import { echoContextFor, type ScannedHole } from './echo-context';

// Only the fields the derivation reads; the wire types carry a great deal more.
const system = (
	id: number,
	over: { alias?: string | null; home?: boolean; statics?: string[] } = {},
): MapSystemView =>
	({
		kind: 'system',
		id,
		alias: over.alias ?? null,
		is_home: over.home ?? false,
		statics: (over.statics ?? []).map((code) => ({ code, dest_class: null })),
	}) as unknown as MapSystemView;

const edge = (from_system: number, to_system: number): MapConnection =>
	({ from_system, to_system }) as unknown as MapConnection;

const hole = (
	id: number,
	code: string | null,
	created_at = '2026-01-01T00:00:00Z',
): ScannedHole => ({
	id,
	code,
	created_at,
});

const derive = (over: Parameters<typeof echoContextFor>[0]) => echoContextFor(over);

describe('echoContextFor', () => {
	it('knows when the hole was scanned in home', () => {
		const systems = [system(1, { home: true }), system(2)];
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: null,
				holesInOrigin: [],
			}).originIsHome,
		).toBe(true);
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 2,
				targetClassId: 5,
				hole: null,
				holesInOrigin: [],
			}).originIsHome,
		).toBe(false);
	});

	it('counts home’s statics, so their branches stay reserved', () => {
		const systems = [system(1, { home: true, statics: ['H296', 'N110'] })];
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: null,
				holesInOrigin: [],
			}).homeStaticCount,
		).toBe(2);
	});

	it('reads the branch letters off home’s links, whichever way the edge points', () => {
		const systems = [
			system(1, { home: true }),
			system(2, { alias: 'a5s' }),
			system(3, { alias: '+b3a' }),
			system(4, { alias: 'c1a' }),
		];
		const connections = [edge(1, 2), edge(3, 1), edge(2, 4)];
		const ctx = derive({
			systems,
			connections,
			originSystemId: 1,
			targetClassId: 5,
			hole: null,
			holesInOrigin: [],
		});
		// 4 hangs off 2, not off home, so its letter is not home's to reuse.
		expect([...ctx.homeBranchLetters].sort()).toEqual(['a', 'b']);
	});

	it('ignores a link whose far side has no alias yet', () => {
		const systems = [system(1, { home: true }), system(2)];
		expect(
			derive({
				systems,
				connections: [edge(1, 2)],
				originSystemId: 1,
				targetClassId: 5,
				hole: null,
				holesInOrigin: [],
			}).homeBranchLetters,
		).toEqual([]);
	});

	it('calls a hole home’s static when its code matches one', () => {
		const systems = [system(1, { home: true, statics: ['H296'] })];
		const h = hole(10, 'H296');
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: h,
				holesInOrigin: [h],
			}).isHomeStatic,
		).toBe(true);
	});

	it('gives the static slot to the first of two holes with the same code', () => {
		const systems = [system(1, { home: true, statics: ['H296'] })];
		const first = hole(10, 'H296', '2026-01-01T10:00:00Z');
		const second = hole(11, 'H296', '2026-01-01T11:00:00Z');
		const both = [first, second];
		const ctx = (h: ScannedHole) =>
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: h,
				holesInOrigin: both,
			}).isHomeStatic;
		expect(ctx(first)).toBe(true);
		expect(ctx(second)).toBe(false);
	});

	it('breaks a tie on the scan time by id, so the answer does not depend on row order', () => {
		const systems = [system(1, { home: true, statics: ['H296'] })];
		const a = hole(10, 'H296');
		const b = hole(11, 'H296');
		const forward = derive({
			systems,
			connections: [],
			originSystemId: 1,
			targetClassId: 5,
			hole: a,
			holesInOrigin: [a, b],
		});
		const reversed = derive({
			systems,
			connections: [],
			originSystemId: 1,
			targetClassId: 5,
			hole: a,
			holesInOrigin: [b, a],
		});
		expect(forward.isHomeStatic).toBe(true);
		expect(reversed.isHomeStatic).toBe(true);
	});

	it('recognises a static whose type was only just picked', () => {
		// Naming a hole as its type is chosen reads the new type, while the signatures
		// around it are still the ones the store last heard about -- including this hole,
		// under whatever it was before. The hole itself has to be counted from the
		// argument, or a static picked after its neighbours reads as a wandering hole and
		// takes the next branch instead of the `s` slot.
		const systems = [system(1, { home: true, statics: ['H296'] })];
		const picked = hole(10, 'H296');
		const stale = { ...picked, code: null };
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: picked,
				holesInOrigin: [stale, hole(11, 'K162'), hole(12, 'V753')],
			}).isHomeStatic,
		).toBe(true);
	});

	it('is not a static when the code is not one of home’s', () => {
		const systems = [system(1, { home: true, statics: ['H296'] })];
		const h = hole(10, 'D382');
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 1,
				targetClassId: 5,
				hole: h,
				holesInOrigin: [h],
			}).isHomeStatic,
		).toBe(false);
	});

	it('is not a static when the hole was scanned somewhere other than home', () => {
		const systems = [system(1, { home: true, statics: ['H296'] }), system(2, { alias: 'a5s' })];
		const h = hole(10, 'H296');
		expect(
			derive({
				systems,
				connections: [],
				originSystemId: 2,
				targetClassId: 5,
				hole: h,
				holesInOrigin: [h],
			}).isHomeStatic,
		).toBe(false);
	});
	it('does not read a hole\u2019s own branch letter as a branch already taken', () => {
		// Recomputing a name after a retype asks the same question again. If the hole is
		// still in the list under its old name, its own letter reads as spoken for and it
		// moves itself along: c6a becomes d6a on a change that was not even a class change.
		// The caller leaves it out, and this is what that looks like from here.
		const home = system(1, { home: true, statics: ['H296'] });
		const mine = system(3, { alias: 'c6a' });
		const sibling = system(2, { alias: 'b5a' });
		const connections = [edge(1, 2), edge(1, 3)];

		const withMine = derive({
			systems: [home, sibling, mine],
			connections,
			originSystemId: 1,
			targetClassId: 6,
			hole: null,
			holesInOrigin: [],
		});
		expect([...withMine.homeBranchLetters].sort()).toEqual(['b', 'c']);

		const withoutMine = derive({
			systems: [home, sibling],
			connections,
			originSystemId: 1,
			targetClassId: 6,
			hole: null,
			holesInOrigin: [],
		});
		expect([...withoutMine.homeBranchLetters].sort()).toEqual(['b']);
	});

	it('still answers on a map with no home system set', () => {
		const systems = [system(1), system(2, { alias: 'a5s' })];
		const ctx = derive({
			systems,
			connections: [edge(1, 2)],
			originSystemId: 1,
			targetClassId: 5,
			hole: hole(10, 'H296'),
			holesInOrigin: [],
		});
		expect(ctx).toMatchObject({
			originIsHome: false,
			isHomeStatic: false,
			homeBranchLetters: [],
			homeStaticCount: 0,
		});
	});
});
