// Deriving the echo scheme's map-wide inputs from what a signature row already has.
//
// The other two schemes need only the parent's alias and the aliases in use. Echo also
// needs to know where home is, what it is connected to, and which hole is home's static,
// because the static owns the reserved `s` slot. All of it comes off the placements and
// edges the row already holds; nothing new is fetched.

import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import type { EchoContext } from './echo';

/** A signature as this module needs it: its wormhole code and when it was scanned. */
export interface ScannedHole {
	id: number;
	/** The wormhole code, e.g. `H296`; null for anything that is not a resolved wormhole. */
	code: string | null;
	created_at: string;
}

/** The first letter of an alias, with the homeward `+` marker stripped. */
function branchLetterOf(alias: string | null | undefined): string | null {
	const letter = (alias ?? '').trim().replace(/^\+/, '').charAt(0).toLowerCase();
	return letter || null;
}

/**
 * Which hole is home's static, when two carry the same code.
 *
 * Nothing in the data says which of two H296s in a C5 with an H296 static is the static
 * one: a wandering H296 looks identical. The earliest scanned wins, and the scout renames
 * it when that turns out to be wrong -- a smaller problem than two holes sharing the `s`
 * slot, which is what happens if every matching code claims it.
 *
 * Ties on `created_at` fall back to the lower id, so the answer does not depend on the
 * order the rows arrived in.
 */
function isFirstScannedWithCode(hole: ScannedHole, holesInOrigin: readonly ScannedHole[]): boolean {
	if (hole.code === null) return false;
	const sameCode = holesInOrigin.filter((other) => other.code === hole.code);
	const first = sameCode.reduce((earliest, other) => {
		if (other.created_at !== earliest.created_at) {
			return other.created_at < earliest.created_at ? other : earliest;
		}
		return other.id < earliest.id ? other : earliest;
	}, sameCode[0]);
	return first?.id === hole.id;
}

/**
 * The echo inputs for a hole scanned in `originSystemId`. A map with no home system set
 * still returns a context: `originIsHome` is false everywhere, so every suggestion
 * inherits its branch from the system it was scanned in, and a hole scanned in an
 * unaliased system gets no name at all. That is the honest answer -- without a home there
 * is no chain to branch from.
 */
export function echoContextFor(input: {
	systems: readonly MapSystemView[];
	connections: readonly MapConnection[];
	originSystemId: number;
	targetClassId: number | null;
	/** The hole being named, and every hole scanned in the same system. */
	hole: ScannedHole | null;
	holesInOrigin: readonly ScannedHole[];
}): EchoContext {
	const home = input.systems.find((system) => system.is_home) ?? null;
	const originIsHome = home !== null && home.id === input.originSystemId;

	const staticCodes =
		home !== null && home.kind === 'system' ? home.statics.map((entry) => entry.code) : [];

	const aliasById = new Map(input.systems.map((system) => [system.id, system.alias]));
	const homeBranchLetters: string[] = [];
	if (home !== null) {
		for (const connection of input.connections) {
			const neighbour =
				connection.from_system === home.id
					? connection.to_system
					: connection.to_system === home.id
						? connection.from_system
						: null;
			if (neighbour === null) continue;
			const letter = branchLetterOf(aliasById.get(neighbour));
			if (letter) homeBranchLetters.push(letter);
		}
	}

	const isHomeStatic =
		originIsHome &&
		input.hole !== null &&
		input.hole.code !== null &&
		staticCodes.includes(input.hole.code) &&
		isFirstScannedWithCode(input.hole, input.holesInOrigin);

	return {
		originIsHome,
		isHomeStatic,
		targetClassId: input.targetClassId,
		homeBranchLetters,
		homeStaticCount: staticCodes.length,
	};
}

/**
 * The neighbour one hop closer to home: this system's parent in the breadth-first tree
 * rooted at home. The hole leading to it is the way back, which the signature list marks
 * with a `+`. Null when there is no home, the system is home, or nothing connects them.
 */
export function parentTowardHome(
	homeSystemId: number | null,
	systemId: number,
	connections: readonly MapConnection[],
): number | null {
	if (homeSystemId === null || homeSystemId === systemId) return null;

	const neighbours = new Map<number, number[]>();
	const link = (from: number, to: number) => {
		const seen = neighbours.get(from);
		if (seen) seen.push(to);
		else neighbours.set(from, [to]);
	};
	for (const connection of connections) {
		link(connection.from_system, connection.to_system);
		link(connection.to_system, connection.from_system);
	}

	const parent = new Map<number, number>();
	const seen = new Set<number>([homeSystemId]);
	const queue = [homeSystemId];
	while (queue.length > 0) {
		const current = queue.shift()!;
		if (current === systemId) return parent.get(current) ?? null;
		for (const next of neighbours.get(current) ?? []) {
			if (seen.has(next)) continue;
			seen.add(next);
			parent.set(next, current);
			queue.push(next);
		}
	}
	return null;
}
