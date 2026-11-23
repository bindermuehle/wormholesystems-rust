// Reading the unmapped holes ("ghosts") off the graph: which connections lead to one,
// what they are called, and which signature identifies them. Pure over the map payloads.

import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import type { Signature } from '$lib/api/types/Signature';
import type { MappedSystem } from '$lib/map/system';

/**
 * The unmapped holes already drawn off `origin`, keyed by connection. Flying one is the
 * moment it stops being a ghost, so a jump resolves the node already there instead of
 * mapping the same system twice.
 */
export function ghostsFrom(
	origin: MappedSystem,
	systems: MapSystemView[],
	connections: MapConnection[],
): Map<number, number> {
	const ghosts = new Map<number, number>();
	for (const c of connections) {
		const other =
			c.from_system === origin.id ? c.to_system : c.to_system === origin.id ? c.from_system : null;
		if (other === null) continue;
		const placement = systems.find((s) => s.id === other);
		if (placement?.kind === 'ghost') ghosts.set(c.id, other);
	}
	return ghosts;
}

/** The names on the ghosts these signatures are drawn as, keyed by signature. */
export function ghostAliases(
	ghosts: Map<number, number>,
	signatures: Signature[],
	systems: MapSystemView[],
): Map<number, string> {
	const named = new Map<number, string>();
	for (const signature of signatures) {
		if (signature.connection_id === null) continue;
		const placement = ghosts.get(signature.connection_id);
		const alias = systems.find((s) => s.id === placement)?.alias;
		if (alias) named.set(signature.id, alias);
	}
	return named;
}

/** The connection already joining two placements, and the signature explaining it. */
export function existingConnection(
	origin: MappedSystem,
	target: MappedSystem,
	connections: MapConnection[],
	sigs: Signature[],
): { connection: MapConnection; signature: Signature | null } | null {
	const connection = connections.find(
		(c) =>
			(c.from_system === origin.id && c.to_system === target.id) ||
			(c.from_system === target.id && c.to_system === origin.id),
	);
	if (!connection) return null;
	return {
		connection,
		signature: sigs.find((s) => s.connection_id === connection.id) ?? null,
	};
}

/**
 * The signature an unmapped hole is known by. A ghost has no name of its own, so the
 * signature its connection is linked to is the only thing that identifies it, and its
 * type the only thing that says what is on the far side.
 */
export function ghostSignatures(
	systems: MapSystemView[],
	connections: MapConnection[],
	sigs: Signature[],
): Map<number, Signature> {
	const out = new Map<number, Signature>();
	const ghosts = new Set(systems.filter((s) => s.kind === 'ghost').map((s) => s.id));
	if (ghosts.size === 0) return out;
	const byConnection = new Map(
		sigs.filter((s) => s.connection_id !== null).map((s) => [s.connection_id!, s]),
	);
	for (const c of connections) {
		const sig = byConnection.get(c.id);
		if (!sig) continue;
		if (ghosts.has(c.to_system)) out.set(c.to_system, sig);
		if (ghosts.has(c.from_system)) out.set(c.from_system, sig);
	}
	return out;
}
