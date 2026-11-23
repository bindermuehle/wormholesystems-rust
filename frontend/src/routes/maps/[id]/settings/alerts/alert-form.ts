// The alert form's pure half: what the fields amount to, and whether they are enough.

import type { AlertDelivery } from '$lib/api/types/AlertDelivery';
import type { AlertKind } from '$lib/api/types/AlertKind';
import type { AlertMention } from '$lib/api/types/AlertMention';
import type { Match } from '$lib/api/types/Match';
import type { JumpShip } from '$lib/api/types/JumpShip';
import type { Rule } from '$lib/api/types/Rule';
import type { SaveAlert } from '$lib/api/types/SaveAlert';
import type { StructureEventKind } from '$lib/api/types/StructureEventKind';
import { isStructureKind } from '$lib/alerts/vocabulary';

/** A kind that watches no system: kills are about who, structures about whose. */
function needsTarget(kind: AlertKind): boolean {
	return kind === 'proximity' || kind === 'jump_range';
}

export interface AlertDraft {
	name: string;
	kind: AlertKind;
	delivery: AlertDelivery;
	webhookId: number | null;
	mention: AlertMention;
	roleRef: number | null;
	target: number | null;
	/** Proximity only: measure from here through the chain instead of from the nearest mapped system. */
	origin: number | null;
	maxJumps: number;
	shipType: JumpShip;
	jdcLevel: number;
	/** Structure fuel only: how many days before the fuel runs out to fire. */
	fuelDays: number;
	/** Structure alerts only: the events to speak about, of the alert's own kind. */
	structureEvents: StructureEventKind[];
	filters: Rule[];
	filterMatch: Match;
}

/** A comma-separated id list as the ids it names; anything not a positive number drops. */
export function parseIds(value: string): number[] {
	return value
		.split(',')
		.map((part) => Number(part.trim()))
		.filter((id) => Number.isFinite(id) && id > 0);
}

/** Whether the draft can be saved: a name, what the kind needs, and a wired delivery. */
export function isValidAlert(draft: AlertDraft): boolean {
	return (
		draft.name.trim().length > 0 &&
		(!needsTarget(draft.kind) || draft.target !== null) &&
		(draft.kind !== 'structure_fuel' ||
			(Number.isInteger(draft.fuelDays) && draft.fuelDays >= 1 && draft.fuelDays <= 30)) &&
		// The fuel alert always has its days to go on; an attack alert with nothing ticked
		// would never speak.
		(draft.kind !== 'structure_attack' || draft.structureEvents.length > 0) &&
		(draft.delivery !== 'webhook' || draft.webhookId !== null) &&
		(draft.mention !== 'role' || draft.roleRef !== null)
	);
}

/** The save body, with every field the kind does not use stripped rather than nulled. */
export function toSaveAlert(draft: AlertDraft): SaveAlert {
	return {
		name: draft.name.trim(),
		kind: draft.kind,
		delivery: draft.delivery,
		map_webhook_id: draft.delivery === 'webhook' ? (draft.webhookId ?? undefined) : undefined,
		map_webhook_role_id: draft.mention === 'role' ? (draft.roleRef ?? undefined) : undefined,
		mention: draft.mention,
		target_solar_system_id: needsTarget(draft.kind) ? (draft.target ?? undefined) : undefined,
		origin_solar_system_id: draft.kind === 'proximity' ? (draft.origin ?? undefined) : undefined,
		// A structure alert measures nothing in jumps; the column still wants a number.
		max_jumps: isStructureKind(draft.kind) ? 0 : draft.maxJumps,
		ship_type: draft.kind === 'jump_range' ? draft.shipType : undefined,
		jdc_level: draft.kind === 'jump_range' ? draft.jdcLevel : undefined,
		fuel_days: draft.kind === 'structure_fuel' ? draft.fuelDays : undefined,
		structure_events: isStructureKind(draft.kind) ? draft.structureEvents : [],
		filters: draft.kind === 'killmail' ? draft.filters.filter((r) => r.ids.length > 0) : [],
		filter_match: draft.filterMatch,
	};
}
