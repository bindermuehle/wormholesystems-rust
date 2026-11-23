// The structures card's words and thresholds: what an event is called, how urgent a fuel
// timer is, and which events are worth interrupting someone for. Pure, and `now` is an
// argument, so the card, the toast and the tests agree.

import type { MapStructure } from '$lib/api/types/MapStructure';
import type { StructureEvent } from '$lib/api/types/StructureEvent';
import type { StructureEventKind } from '$lib/api/types/StructureEventKind';

/** How loud an event is: somebody is shooting, the fuel is going, or nothing is wrong. */
export type EventTone = 'attack' | 'fuel' | 'routine';

export interface StructureEventMeta {
	kind: StructureEventKind;
	/** What an alert's checkbox calls it. */
	label: string;
	/** What happened, finishing the sentence "<structure> …". The same words Discord gets. */
	verb: string;
	/** Which of the two structure alerts speaks about it. */
	alert: 'structure_attack' | 'structure_fuel';
	tone: EventTone;
	/** Whether an alert that was given no choice speaks about it. Mirrors the server. */
	byDefault: boolean;
}

/** Every structure event, once, in the order the alert form lists them. */
export const STRUCTURE_EVENTS: StructureEventMeta[] = [
	{
		kind: 'under_attack',
		label: 'Under attack',
		verb: 'is under attack',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'lost_shields',
		label: 'Lost shields',
		verb: 'lost its shields',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'lost_armor',
		label: 'Lost armor',
		verb: 'lost its armor',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'reinforced',
		label: 'Reinforced (customs office, mercenary den)',
		verb: 'was reinforced',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'destroyed',
		label: 'Destroyed',
		verb: 'was destroyed',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'unanchoring',
		label: 'Unanchoring started',
		verb: 'is being unanchored',
		alert: 'structure_attack',
		tone: 'attack',
		byDefault: true,
	},
	{
		kind: 'anchoring',
		label: 'Anchoring started',
		verb: 'started anchoring',
		alert: 'structure_attack',
		tone: 'routine',
		byDefault: false,
	},
	{
		kind: 'online',
		label: 'Came online',
		verb: 'came online',
		alert: 'structure_attack',
		tone: 'routine',
		byDefault: false,
	},
	{
		kind: 'fuel_low',
		label: "EVE's low fuel warning",
		verb: 'is running out of fuel',
		alert: 'structure_fuel',
		tone: 'fuel',
		byDefault: true,
	},
	{
		kind: 'low_power',
		label: 'Ran out of fuel',
		verb: 'ran out of fuel',
		alert: 'structure_fuel',
		tone: 'fuel',
		byDefault: true,
	},
	{
		kind: 'reagents_low',
		label: 'Metenox low on reagents',
		verb: 'is running out of reagents',
		alert: 'structure_fuel',
		tone: 'fuel',
		byDefault: true,
	},
	{
		kind: 'services_offline',
		label: 'Services went offline',
		verb: 'had its services go offline',
		alert: 'structure_fuel',
		tone: 'fuel',
		byDefault: false,
	},
	{
		kind: 'high_power',
		label: 'Refuelled after running dry',
		verb: 'is back at full power',
		alert: 'structure_fuel',
		tone: 'routine',
		byDefault: false,
	},
];

const META = Object.fromEntries(STRUCTURE_EVENTS.map((e) => [e.kind, e])) as Record<
	StructureEventKind,
	StructureEventMeta
>;

export function eventMeta(kind: StructureEventKind): StructureEventMeta {
	return META[kind];
}

/** The events one of the two alerts can speak about. */
export function eventsOf(alert: StructureEventMeta['alert']): StructureEventMeta[] {
	return STRUCTURE_EVENTS.filter((e) => e.alert === alert);
}

/** What an alert speaks about before anyone chose: the bad news, not the routine. */
export function defaultEvents(alert: StructureEventMeta['alert']): StructureEventKind[] {
	return eventsOf(alert)
		.filter((e) => e.byDefault)
		.map((e) => e.kind);
}

export const EVENT_TONE_CLASS = {
	attack: 'text-red-400',
	fuel: 'text-amber-400',
	routine: 'text-emerald-400',
} satisfies Record<EventTone, string>;

/** Its own name, else its type, else the least there is to say. */
export function structureLabel(of: {
	structure_name?: string | null;
	name?: string | null;
	type_name?: string | null;
}): string {
	return of.structure_name ?? of.name ?? of.type_name ?? 'A structure';
}

/** "J123456 - Home is under attack", for a toast or a row's tooltip. */
export function eventSentence(event: StructureEvent): string {
	return `${structureLabel(event)} ${eventMeta(event.kind).verb}`;
}

/** Who is shooting, as far as the notification said: "Bad Guy (Evictors)". */
export function aggressorLabel(event: StructureEvent): string | null {
	const org = event.aggressor_corporation ?? event.aggressor_alliance ?? null;
	if (event.aggressor && org) return `${event.aggressor} (${org})`;
	return event.aggressor ?? org;
}

export type FuelTone = 'out' | 'critical' | 'low' | 'fine' | 'unfueled' | 'unknown';

/** Not online yet, so having no fuel timer does not mean it ran dry. */
const NOT_ONLINE_YET = [
	'anchoring',
	'anchor_vulnerable',
	'deploy_vulnerable',
	'onlining_vulnerable',
];

/** The states that are a timer or a window, in the words the card uses. */
const REINFORCED: Record<string, string> = {
	anchoring: 'anchoring',
	anchor_vulnerable: 'anchoring, vulnerable',
	deploy_vulnerable: 'deploying, vulnerable',
	onlining_vulnerable: 'needs a quantum core',
	armor_reinforce: 'armor timer',
	armor_vulnerable: 'armor vulnerable',
	hull_reinforce: 'hull timer',
	hull_vulnerable: 'hull vulnerable',
};

/** What a structure is waiting on besides fuel, and until when, if anything. */
export function stateNote(structure: MapStructure): { label: string; until: string | null } | null {
	const label = REINFORCED[structure.state];
	return label ? { label, until: structure.state_timer_end ?? null } : null;
}

const DAY_MS = 24 * 60 * 60 * 1000;
/** Under this, somebody has to haul today. */
export const FUEL_CRITICAL_MS = 3 * DAY_MS;
/** Under this, it belongs on this week's list. */
export const FUEL_LOW_MS = 7 * DAY_MS;

/**
 * How urgent a structure's fuel is. One ESI lists without a timer has run dry, unless it
 * is still anchoring and was never fuelled.
 */
export function fuelTone(structure: MapStructure, now: Date): FuelTone {
	if (!structure.fuel_expires) {
		return NOT_ONLINE_YET.includes(structure.state) ? 'unfueled' : 'out';
	}
	const left = new Date(structure.fuel_expires).getTime() - now.getTime();
	if (Number.isNaN(left)) return 'unknown';
	if (left <= 0) return 'out';
	if (left < FUEL_CRITICAL_MS) return 'critical';
	if (left < FUEL_LOW_MS) return 'low';
	return 'fine';
}

export const FUEL_TONE_CLASS = {
	out: 'text-red-400',
	critical: 'text-red-400',
	low: 'text-amber-400',
	fine: 'text-muted-foreground',
	unfueled: 'text-muted-foreground',
	unknown: 'text-muted-foreground',
} satisfies Record<FuelTone, string>;

/** Milliseconds of fuel left, or null when there is no timer to count down. */
export function fuelLeftMs(structure: MapStructure, now: Date): number | null {
	if (!structure.fuel_expires) return null;
	const left = new Date(structure.fuel_expires).getTime() - now.getTime();
	return Number.isNaN(left) || left <= 0 ? null : left;
}

/** An event older than this is history: it goes on the card, not in front of the map. */
export const NEWS_MS = 30 * 60 * 1000;

/**
 * The events to toast after a refetch: ones not seen before that only just happened.
 * `seen` is null on the first load, when everything already on the card is old news.
 */
export function fresh(
	events: StructureEvent[],
	seen: ReadonlySet<number> | null,
	now: Date,
): StructureEvent[] {
	if (seen === null) return [];
	return events.filter(
		(event) =>
			!seen.has(event.id) && now.getTime() - new Date(event.occurred_at).getTime() < NEWS_MS,
	);
}

/**
 * Whether a row answers a search: every word typed has to be somewhere in what the row
 * shows, the map's own alias for the system included, in any order and any case.
 */
export function matchesQuery(
	row: {
		structure_name?: string;
		name?: string;
		type_name?: string;
		system_name?: string;
		region?: string;
		corporation_name?: string;
		aggressor?: string;
		aggressor_corporation?: string;
		aggressor_alliance?: string;
	},
	query: string,
	alias: string | null = null,
): boolean {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	if (words.length === 0) return true;
	const haystack = [
		row.structure_name,
		row.name,
		row.type_name,
		row.system_name,
		alias,
		row.region,
		row.corporation_name,
		row.aggressor,
		row.aggressor_corporation,
		row.aggressor_alliance,
	]
		.filter(Boolean)
		.join(' ')
		.toLowerCase();
	return words.every((word) => haystack.includes(word));
}

/**
 * How long until a timer runs out, or null once it has (or there is none). The list ESI
 * gives us is up to an hour old, so a timer that has passed says nothing rather than
 * counting up.
 */
export function timerLeftMs(until: string | null | undefined, now: Date): number | null {
	if (!until) return null;
	const left = new Date(until).getTime() - now.getTime();
	return Number.isNaN(left) || left <= 0 ? null : left;
}
