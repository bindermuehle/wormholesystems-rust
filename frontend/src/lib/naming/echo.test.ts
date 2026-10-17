import { describe, expect, it } from 'vitest';
import { classMeta } from '$lib/map/classes';
import { guessNextAlias, suggestAlias } from './alias';
import {
	echoTypeChar,
	generateEchoAlias,
	knownWormholeClassIds,
	nextBranchLetter,
	nextSlotLetter,
	type EchoContext,
} from './echo';

const ctx = (over: Partial<EchoContext> = {}): EchoContext => ({
	originIsHome: false,
	isHomeStatic: false,
	targetClassId: 5,
	homeBranchLetters: [],
	homeStaticCount: 0,
	...over,
});

describe('echoTypeChar', () => {
	it('names the wormhole classes by number', () => {
		expect([1, 2, 3, 4, 5, 6].map(echoTypeChar)).toEqual(['1', '2', '3', '4', '5', '6']);
	});

	it('reserves letters for k-space and Pochven', () => {
		expect(echoTypeChar(7)).toBe('H');
		expect(echoTypeChar(8)).toBe('L');
		expect(echoTypeChar(9)).toBe('N');
		expect(echoTypeChar(25)).toBe('P');
	});

	it('gives Thera T and every drifter class D', () => {
		expect(echoTypeChar(12)).toBe('T');
		expect([14, 15, 16, 17, 18].map(echoTypeChar)).toEqual(['D', 'D', 'D', 'D', 'D']);
	});

	it('keeps the number for shattered, since S reads as the static slot', () => {
		expect(echoTypeChar(13)).toBe('13');
	});

	it('names nothing for abyssal classes or an unknown one', () => {
		expect([19, 20, 21, 22, 23].map(echoTypeChar)).toEqual([null, null, null, null, null]);
		expect(echoTypeChar(null)).toBeNull();
		expect(echoTypeChar(99)).toBeNull();
	});

	// The table in echo.ts hard-codes CCP's class ids. If the class catalogue grows a
	// class the scheme does not name, this says so rather than letting it silently
	// produce no suggestion.
	it('names every class the map can draw, except the abyssal ones', () => {
		const unnamed = knownWormholeClassIds().filter((id) => echoTypeChar(id) === null);
		expect(unnamed).toEqual([19, 20, 21, 22, 23]);
	});

	it('agrees with the class catalogue on which classes are drifters', () => {
		const drifters = knownWormholeClassIds().filter((id) => classMeta(id, null).isDrifter);
		expect(drifters.every((id) => echoTypeChar(id) === 'D')).toBe(true);
	});
});

describe('nextBranchLetter', () => {
	it('starts at a', () => {
		expect(nextBranchLetter([])).toBe('a');
	});

	it('fills a gap left by a branch that rolled out', () => {
		expect(nextBranchLetter(['a', 'c'])).toBe('b');
	});

	it('ignores case', () => {
		expect(nextBranchLetter(['A', 'B'])).toBe('c');
	});
});

describe('nextSlotLetter', () => {
	it('starts at a and skips the static slot', () => {
		expect(nextSlotLetter('a', '5', [])).toBe('a');
		expect(nextSlotLetter('a', '5', ['a5a', 'a5b'])).toBe('c');
	});

	it('never hands out s', () => {
		const taken = 'abcdefghijklmnopqr'.split('').map((l) => `a5${l}`);
		expect(nextSlotLetter('a', '5', taken)).toBe('t');
	});

	it('counts only its own branch and type', () => {
		expect(nextSlotLetter('a', '5', ['b5a', 'a4a', 'aHa'])).toBe('a');
	});

	it('ignores the homeward + marker', () => {
		expect(nextSlotLetter('a', '5', ['+a5a'])).toBe('b');
	});

	it('does not confuse a shattered alias with a C1 slot', () => {
		// `a13a` is branch a, class 13, slot a -- not a C1 holding slot 3.
		expect(nextSlotLetter('a', '1', ['a13a'])).toBe('a');
		expect(nextSlotLetter('a', '13', ['a13a'])).toBe('b');
	});
});

describe('generateEchoAlias', () => {
	it("gives home's static the reserved s slot", () => {
		expect(generateEchoAlias(null, [], ctx({ originIsHome: true, isHomeStatic: true }))).toBe(
			'a5s',
		);
	});

	it('starts a wandering hole off home past the reserved static branches', () => {
		expect(generateEchoAlias(null, [], ctx({ originIsHome: true, homeStaticCount: 1 }))).toBe(
			'b5a',
		);
	});

	it('inherits the branch letter from the system scanned in', () => {
		expect(generateEchoAlias('b4a', ['b4a'], ctx({ targetClassId: 3 }))).toBe('b3a');
	});

	it('keeps the branch at any depth', () => {
		expect(generateEchoAlias('b3c', ['b4a', 'b3c'], ctx({ targetClassId: 6 }))).toBe('b6a');
	});

	it('names nothing when there is no branch to inherit', () => {
		expect(generateEchoAlias(null, [], ctx())).toBeNull();
		expect(generateEchoAlias('   ', [], ctx())).toBeNull();
	});

	it('names nothing for a class the scheme does not cover', () => {
		expect(generateEchoAlias('a5a', [], ctx({ targetClassId: 19 }))).toBeNull();
	});

	it('runs shattered out to four characters', () => {
		expect(generateEchoAlias('a5a', ['a5a'], ctx({ targetClassId: 13 }))).toBe('a13a');
	});

	it('gives Thera T and a drifter D', () => {
		expect(generateEchoAlias('a5a', [], ctx({ targetClassId: 12 }))).toBe('aTa');
		expect(generateEchoAlias('a5a', [], ctx({ targetClassId: 16 }))).toBe('aDa');
	});

	it('shares one slot sequence across the drifter classes', () => {
		// A Sentinel named aDa pushes a later Redoubt on that branch to aDb.
		expect(generateEchoAlias('a5a', ['aDa'], ctx({ targetClassId: 18 }))).toBe('aDb');
	});

	it('names nothing once a branch and type have no slot left', () => {
		const taken = 'abcdefghijklmnopqrtuvwxyz'.split('').map((l) => `a5${l}`);
		expect(generateEchoAlias('a5a', taken, ctx())).toBeNull();
	});
});

describe('guessNextAlias with the echo scheme', () => {
	it('does not upper-case the alias', () => {
		const alias = guessNextAlias(null, [], {
			scheme: 'echo',
			echo: ctx({ originIsHome: true, isHomeStatic: true }),
		});
		expect(alias).toBe('a5s');
	});

	it('falls back to the numeric sequence when no echo context is given', () => {
		expect(guessNextAlias('1', ['1'], { scheme: 'echo' })).toBe('11');
	});
});

describe('suggestAlias with the echo scheme', () => {
	it('names a hole scanned in home, which has no alias to inherit', () => {
		const alias = suggestAlias({
			parentAlias: null,
			targetIsWormhole: true,
			originIsWormhole: true,
			aliases: [],
			scheme: 'echo',
			echo: ctx({ originIsHome: true, isHomeStatic: true }),
		});
		expect(alias).toBe('a5s');
	});

	it('returns null rather than an empty alias for a class it does not name', () => {
		const alias = suggestAlias({
			parentAlias: 'a5a',
			targetIsWormhole: true,
			originIsWormhole: true,
			aliases: [],
			scheme: 'echo',
			echo: ctx({ targetClassId: 20 }),
		});
		expect(alias).toBeNull();
	});

	it('still declines a k-space hop that is not part of the chain', () => {
		const alias = suggestAlias({
			parentAlias: null,
			targetIsWormhole: false,
			originIsWormhole: false,
			aliases: [],
			scheme: 'echo',
			echo: ctx({ targetClassId: 7 }),
		});
		expect(alias).toBeNull();
	});
});
