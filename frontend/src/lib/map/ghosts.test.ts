import { describe, expect, it } from 'vitest';

import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import type { Signature } from '$lib/api/types/Signature';
import type { MappedSystem } from '$lib/map/system';
import { existingConnection, ghostAliases, ghostSignatures, ghostsFrom } from './ghosts';

const system = (id: number): MapSystemView => ({ kind: 'system', id }) as MapSystemView;
const ghost = (id: number, alias: string | null = null): MapSystemView =>
	({ kind: 'ghost', id, alias }) as MapSystemView;
const conn = (id: number, from: number, to: number): MapConnection =>
	({ id, from_system: from, to_system: to }) as MapConnection;
const sig = (id: number, connectionId: number | null, code = 'ABC-123'): Signature =>
	({ id, connection_id: connectionId, signature_id: code }) as Signature;

const ORIGIN = system(1) as MappedSystem;
const SYSTEMS = [ORIGIN, system(2), ghost(3, '1a'), ghost(4)];

describe('ghostsFrom', () => {
	it('finds ghosts on either end of a connection off the origin', () => {
		const ghosts = ghostsFrom(ORIGIN, SYSTEMS, [conn(10, 1, 3), conn(11, 4, 1), conn(12, 1, 2)]);
		expect(ghosts).toEqual(
			new Map([
				[10, 3],
				[11, 4],
			]),
		);
	});

	it('ignores connections that do not touch the origin', () => {
		expect(ghostsFrom(ORIGIN, SYSTEMS, [conn(10, 2, 3)]).size).toBe(0);
	});
});

describe('ghostAliases', () => {
	it('names a signature after the ghost its connection is drawn as', () => {
		const ghosts = new Map([[10, 3]]);
		const named = ghostAliases(ghosts, [sig(7, 10), sig(8, null)], SYSTEMS);
		expect(named).toEqual(new Map([[7, '1a']]));
	});

	it('says nothing for a ghost without an alias', () => {
		const ghosts = new Map([[11, 4]]);
		expect(ghostAliases(ghosts, [sig(7, 11)], SYSTEMS).size).toBe(0);
	});
});

describe('existingConnection', () => {
	const target = system(2) as MappedSystem;

	it('finds the connection in either direction, with its signature', () => {
		const c = conn(12, 2, 1);
		const s = sig(7, 12);
		expect(existingConnection(ORIGIN, target, [c], [s])).toEqual({ connection: c, signature: s });
	});

	it('reports a connection without a signature as unexplained', () => {
		expect(existingConnection(ORIGIN, target, [conn(12, 1, 2)], [])).toEqual({
			connection: conn(12, 1, 2),
			signature: null,
		});
	});

	it('is nothing when the two are not joined', () => {
		expect(existingConnection(ORIGIN, target, [conn(12, 1, 3)], [])).toBeNull();
	});
});

describe('ghostSignatures', () => {
	it('keys each ghost by the signature its connection is linked to, both endpoints', () => {
		const sigs = [sig(7, 10, 'AAA-111'), sig(8, 11, 'BBB-222')];
		const out = ghostSignatures(SYSTEMS, [conn(10, 1, 3), conn(11, 4, 1)], sigs);
		expect(out).toEqual(
			new Map([
				[3, sigs[0]],
				[4, sigs[1]],
			]),
		);
	});

	it('leaves an unscanned ghost unnamed', () => {
		expect(ghostSignatures(SYSTEMS, [conn(10, 1, 3)], []).size).toBe(0);
	});
});
