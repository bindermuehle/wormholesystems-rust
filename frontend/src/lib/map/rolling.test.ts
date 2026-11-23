import { describe, expect, it } from 'vitest';

import { describeJumps, jumpOrder, massClasses, rollPlan, type RollSteps } from './rolling';

const GG = 1_000_000;
/** The battleship the old quick reference chart assumes. */
const battleship = { cold: 200 * GG, hot: 300 * GG };
const plan = (total: number, limit: number | null = 375) =>
	rollPlan(total * GG, limit === null ? null : limit * GG, battleship)!;
const jumps = (steps: RollSteps) => describeJumps(steps.jumps);

describe('rollPlan', () => {
	// The chart this is built from, by Algar Thiesant: the same steps for the same ship.
	it('rolls a 1000 hole as the chart does', () => {
		const p = plan(1000);
		expect(describeJumps(p.first)).toBe('1 cold, 1 hot');
		expect(p.away).toBe(false);
		expect(jumps(p.reduced!.roll)).toBe('2 hot');
		expect(jumps(p.reduced!.crit)).toBe('2 cold');
		expect(jumps(p.fresh!.roll)).toBe('2 hot');
		expect(jumps(p.fresh!.crit)).toBe('1 cold, 1 hot');
	});

	it('rolls a 2000 hole as the chart does', () => {
		const p = plan(2000);
		expect(describeJumps(p.first)).toBe('2 cold, 2 hot');
		expect(jumps(p.reduced!.roll)).toBe('2 cold, 2 hot');
		expect(jumps(p.reduced!.crit)).toBe('4 cold');
		expect(jumps(p.fresh!.roll)).toBe('4 hot');
		expect(jumps(p.fresh!.crit)).toBe('2 cold, 2 hot');
		expect(p.reduced!.roll.certain).toBe(true);
		expect(p.fresh!.roll.certain).toBe(true);
	});

	it('rolls a 3000 hole as the chart does, from the far side', () => {
		const p = plan(3000);
		expect(describeJumps(p.first)).toBe('5 hot');
		expect(p.away).toBe(true);
		// "Return hot, 4 hot", and "return hot, 3 cold, 3 hot".
		expect(jumps(p.reduced!.roll)).toBe('5 hot');
		expect(jumps(p.fresh!.roll)).toBe('3 cold, 4 hot');
		// "Return cold, 2 cold, 2 hot", and "return hot, 4 hot".
		expect(jumps(p.reduced!.crit)).toBe('3 cold, 2 hot');
		expect(jumps(p.fresh!.crit)).toBe('5 hot');
	});

	it('leaves a 3300 hole critical as the chart does', () => {
		const p = plan(3300, 2000);
		expect(describeJumps(p.first)).toBe('1 cold, 5 hot');
		expect(jumps(p.reduced!.crit)).toBe('4 hot');
		expect(jumps(p.fresh!.crit)).toBe('1 cold, 5 hot');
	});

	it('says so when no number of jumps is sure to close the hole', () => {
		const p = plan(3300, 2000);
		// The chart risks the roller here ("2 cold, 4 hot"); this stops short of that, which
		// leaves the hole critical for a light ship to finish.
		expect(jumps(p.reduced!.roll)).toBe('3 cold, 3 hot');
		expect(p.reduced!.roll.certain).toBe(false);
		expect(p.reduced!.roll.critical).toBe(true);
		// Where the chart says "6 hot, HIC once if needed", two more jumps make it sure.
		expect(jumps(p.fresh!.roll)).toBe('4 cold, 4 hot');
		expect(p.fresh!.roll.certain).toBe(true);
	});

	it('never has the hole close with the roller on the far side', () => {
		for (const [total, limit] of [
			[750, 375],
			[1000, 375],
			[2000, 375],
			[2000, 1000],
			[3000, 375],
			[3000, 1000],
			[3300, 2000],
			[5000, 375],
		]) {
			const p = plan(total, limit);
			const first = p.first.cold * 200 + p.first.hot * 300;
			for (const [branch, low] of [
				[p.reduced, total * 0.9],
				[p.fresh, Math.max(total * 0.9, first * 2)],
			] as const) {
				if (!branch) continue;
				const { jumps: j, last } = branch.roll;
				const through = first + j.cold * 200 + j.hot * 300;
				const lightest = last === 'cold' || j.hot === 0 ? 200 : 300;
				expect(through - lightest, `${total}`).toBeLessThanOrEqual(low);
				// And every plan ends at home.
				const count = p.first.cold + p.first.hot + j.cold + j.hot;
				expect(count % 2, `${total}`).toBe(0);
				const crit = branch.crit.jumps;
				expect(first + crit.cold * 200 + crit.hot * 300, `${total}`).toBeLessThanOrEqual(low);
			}
		}
	});

	it('uses only what fits through the hole', () => {
		// 300 does not fit a 250 limit, so every jump is cold.
		const p = rollPlan(1000 * GG, 250 * GG, battleship)!;
		expect(p.first.hot).toBe(0);
		// No number of 200s lands near half of 1000, so the hole has only one answer to give.
		expect(p.reduced).toBeNull();
		expect(p.fresh!.roll.jumps.hot).toBe(0);
		expect(p.fresh!.roll.jumps.cold).toBeGreaterThan(0);
	});

	it('has no plan for a hole the ship cannot enter', () => {
		expect(rollPlan(500 * GG, 62 * GG, battleship)).toBeNull();
	});
});

describe('massClasses', () => {
	it('groups the codes by mass and leaves out a hole with none', () => {
		const type = (signature: string | null, total_mass: number | null, max_jump_mass = 375) => ({
			signature,
			total_mass,
			max_jump_mass,
		});
		expect(
			massClasses([
				type('H296', 3300, 2000),
				type('C247', 2000),
				type('K162', null),
				type('A239', 2000),
				// The same hole is listed once per place it spawns.
				type('C247', 2000),
				type(null, null),
			]),
		).toEqual([
			{ totalMass: 2000, maxJump: 375, codes: ['A239', 'C247'] },
			{ totalMass: 3300, maxJump: 2000, codes: ['H296'] },
		]);
	});
});

describe('jumpOrder', () => {
	const flown = (...args: Parameters<typeof jumpOrder>) =>
		jumpOrder(...args).map((j) => `${j.back ? 'back' : 'out'} ${j.mass}`);

	it('goes out and back, cold before hot', () => {
		expect(flown({ cold: 2, hot: 2 }, null, false)).toEqual([
			'out cold',
			'back cold',
			'out hot',
			'back hot',
		]);
	});

	it('ends on the mass the last jump has to be', () => {
		expect(flown({ cold: 1, hot: 1 }, 'cold', false)).toEqual(['out hot', 'back cold']);
	});

	it('starts with the way home from the far side', () => {
		expect(flown({ cold: 0, hot: 3 }, null, true)).toEqual(['back hot', 'out hot', 'back hot']);
	});

	it('is empty for no jumps', () => {
		expect(jumpOrder({ cold: 0, hot: 0 }, null, false)).toEqual([]);
	});
});
