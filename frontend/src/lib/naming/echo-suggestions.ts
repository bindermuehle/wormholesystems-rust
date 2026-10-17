// The alias the signature list shows per row.
//
// The suggestions are worked out for the whole list in one pass rather than per row,
// because each one takes a slot: two unscanned C5s off the same branch are `a5a` and
// `a5b`, and a row deciding on its own would call both of them `a5a`. The pass walks the
// holes in scan order and adds each name it hands out to the pool the next one reads.

import type { MapConnection } from '$lib/api/types/MapConnection';
import type { MapSystemView } from '$lib/api/types/MapSystemView';
import type { Signature } from '$lib/api/types/Signature';
import type { SignatureCatalog } from '$lib/api/types/SignatureCatalog';
import { isWormholeClass } from '$lib/map/classes';
import { typeById } from '$lib/map/signatures';
import { suggestAlias } from './alias';
import { echoContextFor, parentTowardHome, type ScannedHole } from './echo-context';

export interface RowAlias {
	/** The name itself, without the homeward marker. */
	alias: string;
	/** This hole is the way back toward home. */
	homeward: boolean;
	/** The destination is on the map and carries this name; otherwise it is a suggestion. */
	settled: boolean;
}

/**
 * The alias for every wormhole row in `system`, by signature id.
 *
 * A hole already linked to a placement reports that placement's name: it is settled, and
 * nothing should suggest a different one. An unlinked hole gets a suggestion, which is
 * only possible once its type says what it leads to -- an unscanned row has no class and
 * so no name.
 */
export function echoSuggestions(input: {
	systems: readonly MapSystemView[];
	connections: readonly MapConnection[];
	system: MapSystemView;
	sigs: readonly Signature[];
	catalog: SignatureCatalog;
	ignoredAlias?: string;
}): Map<number, RowAlias> {
	const out = new Map<number, RowAlias>();
	const home = input.systems.find((s) => s.is_home) ?? null;
	const homeward = parentTowardHome(home?.id ?? null, input.system.id, input.connections);

	const systemById = new Map(input.systems.map((s) => [s.id, s]));
	const connectionById = new Map(input.connections.map((c) => [c.id, c]));

	// Every name on the map, which each suggestion must avoid and each one taken adds to.
	const pool = input.systems.map((s) => s.alias).filter((alias): alias is string => Boolean(alias));

	const asHole = (sig: Signature): ScannedHole => ({
		id: sig.id,
		code: typeById(input.catalog, sig.signature_type_id)?.signature ?? null,
		created_at: sig.created_at,
	});
	const holesInOrigin = input.sigs.filter((s) => s.group === 'wormhole').map(asHole);

	// Scan order, not display order: which hole took a slot must not change when the list
	// is sorted by age or type.
	const inScanOrder = [...input.sigs]
		.filter((sig) => sig.group === 'wormhole')
		.sort((a, b) => a.created_at.localeCompare(b.created_at) || a.id - b.id);

	for (const sig of inScanOrder) {
		const connection = sig.connection_id === null ? null : connectionById.get(sig.connection_id);
		const farId = connection
			? connection.from_system === input.system.id
				? connection.to_system
				: connection.from_system
			: null;
		const far = farId === null ? null : (systemById.get(farId) ?? null);

		// A destination the chain has already named keeps that name; nothing should
		// suggest a different one for a system people are flying to.
		if (far?.alias) {
			out.set(sig.id, { alias: far.alias, homeward: farId === homeward, settled: true });
			continue;
		}

		// Everything else gets a suggestion, linked or not. A hole drawn as a ghost is
		// linked from the moment it is scanned but has no name until someone gives it one,
		// and that is exactly the row most in need of one.
		//
		// Where the far side is a real system its own class is the truth; a ghost has none,
		// so the signature's type says what it promises to lead to.
		const targetClassId =
			(far?.kind === 'system' ? far.wormhole_class_id : null) ??
			typeById(input.catalog, sig.signature_type_id)?.target_class ??
			null;
		// Until the type is known the hole leads nowhere in particular, and the scheme
		// names a destination by its class.
		if (targetClassId === null) continue;

		const alias = suggestAlias({
			parentAlias: input.system.alias,
			targetIsWormhole: isWormholeClass(targetClassId),
			originIsWormhole:
				input.system.kind === 'system' && isWormholeClass(input.system.wormhole_class_id),
			aliases: pool,
			scheme: 'echo',
			ignoredAlias: input.ignoredAlias,
			echo: echoContextFor({
				systems: input.systems,
				connections: input.connections,
				originSystemId: input.system.id,
				targetClassId,
				hole: asHole(sig),
				holesInOrigin,
			}),
		});
		if (!alias) continue;

		out.set(sig.id, { alias, homeward: farId !== null && farId === homeward, settled: false });
		// Taken, so the next unscanned hole of this class gets the following slot.
		pool.push(alias);
	}

	return out;
}
