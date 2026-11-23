// How to roll a wormhole with one ship: what to put through a fresh hole, what its answer
// to "is it reduced?" then says about its mass, and what closes it or leaves it critical
// from there.
//
// A hole's total mass is its type's figure give or take 10%, and nobody is told which. It
// shows reduced once more than half of the real total has gone through, critical once more
// than nine tenths has, and it is gone when all of it has. So putting half the nominal
// mass through and looking at the hole halves the doubt: reduced means it is on the small
// side, not reduced that it is on the big side.
//
// Every plan leaves the roller at home. A hole may only close on a jump home, so what has
// gone through before the last jump has to be no more than the smallest the hole can be.
// The order of the jumps before that does not matter.

/** How far a hole's real mass strays from its type's, either way. */
const VARIANCE = 0.1;
/** The share gone at which a hole shows reduced, and at which it shows critical. */
const REDUCED = 0.5;
const CRITICAL = 0.9;
/** More jumps than this is not a plan anyone flies; the search stops there. */
const MAX_JUMPS = 16;

/** The rolling ship, in kg: propulsion module off, and on. */
export interface RollingShip {
	cold: number;
	hot: number;
}

export interface Jumps {
	cold: number;
	hot: number;
}

export interface RollSteps {
	jumps: Jumps;
	/** The mass the last jump home has to be flown at, when it matters which. */
	last: 'cold' | 'hot' | null;
	/** Whether these jumps do it for every mass the hole can have. */
	certain: boolean;
	/**
	 * For a roll that is not certain: whether the hole is at least sure to be critical
	 * afterwards, so one light ship finishes it.
	 */
	critical: boolean;
}

/** What to do once the hole has answered. */
export interface RollBranch {
	roll: RollSteps;
	crit: RollSteps;
}

export interface RollPlan {
	/** What goes through first. */
	first: Jumps;
	/** Whether those jumps leave the roller on the far side. */
	away: boolean;
	/** `null` where the hole cannot give that answer after the first jumps. */
	reduced: RollBranch | null;
	fresh: RollBranch | null;
}

interface Candidate {
	cold: number;
	hot: number;
	mass: number;
	count: number;
}

/** Every mix of cold and hot jumps the ship can make through a hole of this jump limit. */
function candidates(ship: RollingShip, maxJump: number | null): Candidate[] {
	const fits = (mass: number) => mass > 0 && (maxJump === null || mass <= maxJump);
	const maxCold = fits(ship.cold) ? MAX_JUMPS : 0;
	const maxHot = fits(ship.hot) ? MAX_JUMPS : 0;
	const all: Candidate[] = [];
	for (let cold = 0; cold <= maxCold; cold++) {
		for (let hot = 0; hot <= maxHot && cold + hot <= MAX_JUMPS; hot++) {
			all.push({ cold, hot, mass: cold * ship.cold + hot * ship.hot, count: cold + hot });
		}
	}
	return all;
}

/** The first by each rule in turn; a rule returns a number, smaller is better. */
function best<T>(items: T[], ...rules: ((item: T) => number)[]): T | null {
	let pick: T | null = null;
	for (const item of items) {
		if (pick === null) {
			pick = item;
			continue;
		}
		for (const rule of rules) {
			const difference = rule(item) - rule(pick);
			if (difference < 0) pick = item;
			if (difference !== 0) break;
		}
	}
	return pick;
}

const none = (certain: boolean): RollSteps => ({
	jumps: { cold: 0, hot: 0 },
	last: null,
	certain,
	critical: false,
});

/** Jumps that close a hole whose mass is somewhere in `low..high`, `passed` already gone. */
function toRoll(
	options: Candidate[],
	ship: RollingShip,
	passed: number,
	away: boolean,
	low: number,
	high: number,
): RollSteps {
	// Each mix, ending on either mass it holds: the last jump is the one that may close it.
	const endings = options.flatMap((c) =>
		(['hot', 'cold'] as const)
			.filter((last) => c[last] > 0)
			.map((last) => ({ ...c, last, before: passed + c.mass - ship[last] })),
	);
	const safe = endings.filter((c) => c.count % 2 === (away ? 1 : 0) && c.before <= low);
	const closing = safe.filter((c) => passed + c.mass >= high);
	// The fewest jumps, then the mix with the most to spare on its tighter side: mass over
	// what closes the biggest hole, and mass under what strands the roller in the smallest.
	const over = (c: { mass: number }) => passed + c.mass - high;
	const under = (c: { before: number }) => low - c.before;
	const sure = best(
		closing,
		(c) => c.count,
		(c) => -Math.min(over(c), under(c)),
		(c) => -under(c),
	);
	if (sure) {
		return {
			jumps: { cold: sure.cold, hot: sure.hot },
			last: sure.cold > 0 && sure.hot > 0 ? sure.last : null,
			certain: true,
			critical: true,
		};
	}
	// Nothing closes it for certain without risking the roller. Go as far as is safe, and
	// prefer ending somewhere the hole is known to be critical.
	const critical = (c: { mass: number }) => passed + c.mass > CRITICAL * high;
	const most = best(
		safe,
		(c) => (critical(c) ? 0 : 1),
		(c) => (critical(c) ? c.count : 0),
		(c) => -c.mass,
		(c) => c.count,
	);
	if (!most) return none(false);
	return {
		jumps: { cold: most.cold, hot: most.hot },
		last: most.cold > 0 && most.hot > 0 ? most.last : null,
		certain: false,
		critical: critical(most),
	};
}

/** Jumps that leave such a hole open and as close to gone as is safe. */
function toCrit(
	options: Candidate[],
	passed: number,
	away: boolean,
	low: number,
	high: number,
): RollSteps {
	const safe = options.filter((c) => c.count % 2 === (away ? 1 : 0) && passed + c.mass <= low);
	const critical = (c: Candidate) => passed + c.mass > CRITICAL * high;
	const pick = best(
		safe,
		(c) => (critical(c) ? 0 : 1),
		(c) => (critical(c) ? c.count : 0),
		(c) => -c.mass,
		(c) => c.count,
	);
	if (!pick) return none(false);
	return {
		jumps: { cold: pick.cold, hot: pick.hot },
		last: null,
		certain: critical(pick),
		critical: critical(pick),
	};
}

/**
 * The plan for a fresh hole of `totalMass` that lets `maxJump` through at a time, or `null`
 * when the ship does not fit through it at all.
 */
export function rollPlan(
	totalMass: number,
	maxJump: number | null,
	ship: RollingShip,
): RollPlan | null {
	const options = candidates(ship, maxJump);
	// Whole kilograms, so a plan that lands exactly on a bound is not lost to rounding.
	const low = Math.round(totalMass * (1 - VARIANCE));
	const high = Math.round(totalMass * (1 + VARIANCE));
	// As near half the nominal mass as the ship can get: that is what splits the doubt
	// evenly. Then fewer jumps, then ending at home, then the heavier of two equal misses.
	const first = best(
		options.filter((c) => c.count > 0 && c.mass <= low),
		(c) => Math.abs(c.mass - totalMass * REDUCED),
		(c) => c.count,
		(c) => c.count % 2,
		(c) => -c.mass,
	);
	if (!first) return null;

	const away = first.count % 2 === 1;
	// Reduced means more than half is gone, so the hole is smaller than twice what went.
	const edge = first.mass / REDUCED;
	const branch = (from: number, to: number): RollBranch | null =>
		from > to
			? null
			: {
					roll: toRoll(options, ship, first.mass, away, from, to),
					crit: toCrit(options, first.mass, away, from, to),
				};
	return {
		first: { cold: first.cold, hot: first.hot },
		away,
		reduced: edge <= low ? null : branch(low, Math.min(high, edge)),
		fresh: edge > high ? null : branch(Math.max(low, edge), high),
	};
}

/** Holes that roll the same way: one total mass and one jump limit. */
export interface MassClass {
	totalMass: number;
	maxJump: number | null;
	/** The wormhole codes of that mass, in order. */
	codes: string[];
}

/** The catalog's wormholes by mass, lightest first. A K162 has no mass of its own. */
export function massClasses(
	types: { signature: string | null; total_mass: number | null; max_jump_mass: number | null }[],
): MassClass[] {
	const classes = new Map<string, MassClass & { seen: Set<string> }>();
	for (const type of types) {
		if (type.signature === null || type.total_mass === null) continue;
		const key = `${type.total_mass}:${type.max_jump_mass}`;
		let entry = classes.get(key);
		if (!entry) {
			entry = {
				totalMass: type.total_mass,
				maxJump: type.max_jump_mass,
				codes: [],
				seen: new Set(),
			};
			classes.set(key, entry);
		}
		if (!entry.seen.has(type.signature)) {
			entry.seen.add(type.signature);
			entry.codes.push(type.signature);
		}
	}
	return [...classes.values()]
		.map(({ totalMass, maxJump, codes }) => ({ totalMass, maxJump, codes: codes.sort() }))
		.sort((a, b) => a.totalMass - b.totalMass || (a.maxJump ?? 0) - (b.maxJump ?? 0));
}

/** One jump of a set: how heavy, and which way. */
export interface Jump {
	mass: 'cold' | 'hot';
	/** Towards home, rather than out to the far side. */
	back: boolean;
}

/**
 * A set of jumps in the order to fly them: the cold ones, then the hot ones, ending on
 * the mass the last jump has to be. Only that last one matters to the plan; the rest is
 * put lightest first so that it reads the same every time. `away` says the roller starts
 * on the far side, which makes the first jump the way home.
 */
export function jumpOrder(jumps: Jumps, last: 'cold' | 'hot' | null, away: boolean): Jump[] {
	if (jumps.cold + jumps.hot === 0) return [];
	const end = last ?? (jumps.hot > 0 ? 'hot' : 'cold');
	const masses: Jump['mass'][] = [
		...Array<'cold'>(jumps.cold - (end === 'cold' ? 1 : 0)).fill('cold'),
		...Array<'hot'>(jumps.hot - (end === 'hot' ? 1 : 0)).fill('hot'),
		end,
	];
	return masses.map((mass, i) => ({ mass, back: i % 2 === (away ? 0 : 1) }));
}

/** "2 cold, 2 hot", or "nothing" for no jumps at all. */
export function describeJumps(jumps: Jumps): string {
	const parts = [];
	if (jumps.cold > 0) parts.push(`${jumps.cold} cold`);
	if (jumps.hot > 0) parts.push(`${jumps.hot} hot`);
	return parts.length > 0 ? parts.join(', ') : 'nothing';
}
