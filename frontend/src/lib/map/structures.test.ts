import { describe, expect, it } from 'vitest';

import type { MapStructure } from '$lib/api/types/MapStructure';
import type { StructureEvent } from '$lib/api/types/StructureEvent';
import {
	aggressorLabel,
	eventSentence,
	fresh,
	fuelLeftMs,
	fuelTone,
	matchesQuery,
	STRUCTURE_EVENTS,
	defaultEvents,
	eventMeta,
	eventsOf,
	stateNote,
	structureLabel,
	timerLeftMs,
} from './structures';

const NOW = new Date('2026-11-16T12:00:00Z');
const HOUR = 3_600_000;

function event(patch: Partial<StructureEvent> = {}): StructureEvent {
	return {
		id: 1,
		kind: 'under_attack',
		structure_name: 'J123456 - Home',
		type_id: 35832,
		type_name: 'Astrahus',
		corporation_id: 98000001,
		occurred_at: new Date(NOW.getTime() - 5 * 60_000).toISOString(),
		...patch,
	};
}

function structure(fuelInHours: number | null): MapStructure {
	return {
		structure_id: 1,
		name: 'J123456 - Home',
		corporation_id: 98000001,
		state: 'shield_vulnerable',
		updated_at: NOW.toISOString(),
		fuel_expires:
			fuelInHours === null ? undefined : new Date(NOW.getTime() + fuelInHours * HOUR).toISOString(),
	};
}

describe('labels', () => {
	it('names a structure by its name, then its type', () => {
		expect(structureLabel(event())).toBe('J123456 - Home');
		expect(structureLabel(event({ structure_name: undefined }))).toBe('Astrahus');
		expect(structureLabel({})).toBe('A structure');
	});

	it('says what happened in the words Discord gets', () => {
		expect(eventSentence(event())).toBe('J123456 - Home is under attack');
		expect(eventSentence(event({ kind: 'low_power' }))).toBe('J123456 - Home ran out of fuel');
	});

	it('names the attacker as far as the notification did', () => {
		expect(aggressorLabel(event())).toBeNull();
		expect(aggressorLabel(event({ aggressor_corporation: 'Evictors' }))).toBe('Evictors');
		expect(aggressorLabel(event({ aggressor: 'Bad Guy', aggressor_corporation: 'Evictors' }))).toBe(
			'Bad Guy (Evictors)',
		);
	});

	it('sorts every event under one alert, and keeps the routine ones opt-in', () => {
		expect(eventMeta('fuel_low').alert).toBe('structure_fuel');
		expect(eventMeta('lost_armor').alert).toBe('structure_attack');
		expect(defaultEvents('structure_attack')).toEqual([
			'under_attack',
			'lost_shields',
			'lost_armor',
			'reinforced',
			'destroyed',
			'unanchoring',
		]);
		expect(defaultEvents('structure_fuel')).toEqual(['fuel_low', 'low_power', 'reagents_low']);
		expect(eventsOf('structure_fuel').map((e) => e.kind)).toContain('high_power');
		// One entry per kind, or a checkbox would be listed twice.
		expect(new Set(STRUCTURE_EVENTS.map((e) => e.kind)).size).toBe(STRUCTURE_EVENTS.length);
	});
});

describe('fuelTone', () => {
	it('grades the time left', () => {
		expect(fuelTone(structure(30 * 24), NOW)).toBe('fine');
		expect(fuelTone(structure(5 * 24), NOW)).toBe('low');
		expect(fuelTone(structure(30), NOW)).toBe('critical');
	});

	it('reads no timer, or one in the past, as out of fuel', () => {
		expect(fuelTone(structure(null), NOW)).toBe('out');
		expect(fuelTone(structure(-1), NOW)).toBe('out');
		expect(fuelLeftMs(structure(null), NOW)).toBeNull();
		// Still anchoring: never fuelled, which is not the same as run dry.
		expect(fuelTone({ ...structure(null), state: 'anchoring' }, NOW)).toBe('unfueled');
		expect(fuelLeftMs(structure(2), NOW)).toBe(2 * HOUR);
	});
});

describe('stateNote', () => {
	it('says which timer a reinforced structure is waiting on', () => {
		expect(stateNote(structure(48))).toBeNull();
		const until = '2026-11-18T20:00:00Z';
		expect(
			stateNote({ ...structure(48), state: 'armor_reinforce', state_timer_end: until }),
		).toEqual({ label: 'armor timer', until });
	});
});

describe('timerLeftMs', () => {
	it('counts down a timer still running and says nothing of one that ran out', () => {
		expect(timerLeftMs(new Date(NOW.getTime() + 2 * HOUR).toISOString(), NOW)).toBe(2 * HOUR);
		expect(timerLeftMs(new Date(NOW.getTime() - HOUR).toISOString(), NOW)).toBeNull();
		expect(timerLeftMs(undefined, NOW)).toBeNull();
	});
});

describe('fresh', () => {
	it('toasts nothing on the first load', () => {
		expect(fresh([event()], null, NOW)).toEqual([]);
	});

	it('toasts what was not there before and only just happened', () => {
		const old = event({ id: 2, occurred_at: new Date(NOW.getTime() - 3 * HOUR).toISOString() });
		const news = event({ id: 3 });
		expect(fresh([event(), old, news], new Set([1]), NOW)).toEqual([news]);
	});
});

describe('matchesQuery', () => {
	const row = event({
		system_name: 'J122515',
		region: 'E-R00024',
		aggressor_corporation: 'Evictors',
	});

	it('matches everything while nothing is typed', () => {
		expect(matchesQuery(row, '  ')).toBe(true);
	});

	it('finds a row by its structure, system, region or attacker, in any case', () => {
		expect(matchesQuery(row, 'home')).toBe(true);
		expect(matchesQuery(row, 'j1225')).toBe(true);
		expect(matchesQuery(row, 'e-r000')).toBe(true);
		expect(matchesQuery(row, 'EVICTORS')).toBe(true);
		expect(matchesQuery(row, 'amarr')).toBe(false);
	});

	it('finds it by what the map calls the system', () => {
		expect(matchesQuery(row, 'staging')).toBe(false);
		expect(matchesQuery(row, 'staging', 'Staging')).toBe(true);
	});

	it('wants every word, in any order', () => {
		expect(matchesQuery(row, 'evictors home')).toBe(true);
		expect(matchesQuery(row, 'evictors jita')).toBe(false);
	});
});
