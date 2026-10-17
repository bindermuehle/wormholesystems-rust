// The ECHO chain naming scheme: `<branch><type><slot>`.
//
// Where the other two schemes extend the parent's alias and so grow with depth, this one
// stays three characters at any depth and says what a hole leads to:
//
//   branch  a letter per direct link off home, inherited by everything down that branch.
//           New links take the lowest free letter; a letter is reused once its branch dies.
//   type    the destination's class: `1`-`6`, `H`/`L`/`N`/`P` for k-space and Pochven,
//           `T` for Thera, `D` for any drifter hole and `13` for shattered.
//   slot    the lowest free letter making `<branch><type>` unique, skipping `s`. Home's
//           own static is the exception and always takes `s`, e.g. `a5s`.
//
// Shattered is the one two-character type, so its aliases run to four (`a13a`). Nothing
// reads an alias by position and a slot is always a letter, so `a13a` cannot be mistaken
// for a C1 in slot `3`, and cannot consume one.

import { classMeta } from '$lib/map/classes';

const BRANCH_LETTERS = 'abcdefghijklmnopqrstuvwxyz'.split('');

/** The map-wide inputs a suggestion needs, beyond the alias of the system scanned in. */
export interface EchoContext {
	/** Whether the system the hole was scanned in is the map's home. */
	originIsHome: boolean;
	/** Whether this hole is one of home's statics, which claims the reserved `s` slot. */
	isHomeStatic: boolean;
	/** `wormhole_class_id` of the destination. */
	targetClassId: number | null;
	/** First letters of the aliases on home's live direct links. */
	homeBranchLetters: readonly string[];
	/**
	 * How many statics home has. Their branches are reserved even before the static is
	 * scanned, so a wandering hole off home never takes the letter the static will want.
	 */
	homeStaticCount: number;
}

/**
 * The scheme's character for a destination class, or null for a class it does not name
 * (abyssal C19-C23, and anything the SDE gives no class).
 *
 * All five drifter classes share `D`. The scheme has one slot for the type and the
 * destination is one click away on the map, so telling Sentinel from Vidette by name is
 * not worth four more reserved letters -- Wanderer labels all five "Drifter" for the same
 * reason. They share one slot sequence too, so a Sentinel named `aDa` pushes a later
 * Redoubt on that branch to `aDb`.
 *
 * Shattered keeps its class number rather than taking a letter, because `S` would read as
 * the reserved `s` static slot. C13 is the only class this applies to: C14-C18 are the
 * drifters above.
 *
 * The ids are CCP's `wormhole_class_id` encoding, the same one `$lib/map/classes` keys on.
 */
export function echoTypeChar(classId: number | null | undefined): string | null {
	if (classId == null) return null;
	if (classId >= 1 && classId <= 6) return String(classId);
	switch (classId) {
		case 7:
			return 'H';
		case 8:
			return 'L';
		case 9:
			return 'N';
		case 12:
			return 'T';
		case 13:
			return '13';
		case 14:
		case 15:
		case 16:
		case 17:
		case 18:
			return 'D';
		case 25:
			return 'P';
		default:
			return null;
	}
}

/** The lowest branch letter not already in use by one of home's links. */
export function nextBranchLetter(used: readonly string[]): string {
	const taken = new Set(used.map((letter) => letter.toLowerCase()));
	return (
		BRANCH_LETTERS.find((letter) => !taken.has(letter)) ?? BRANCH_LETTERS[BRANCH_LETTERS.length - 1]
	);
}

/**
 * The lowest free slot letter for a branch and type, read off every alias on the map.
 * `s` is skipped: it belongs to home's static. Null once the other 25 are gone.
 *
 * The leading `+` that marks the way home is stripped first -- it is a display mark on an
 * existing alias, not part of the name.
 */
export function nextSlotLetter(
	branch: string,
	type: string,
	aliases: readonly string[],
): string | null {
	const prefix = `${branch}${type}`;
	const used = new Set<string>();
	for (const alias of aliases) {
		const name = alias.trim().replace(/^\+/, '');
		if (name.length > prefix.length && name.startsWith(prefix)) {
			used.add(name.charAt(prefix.length));
		}
	}
	return BRANCH_LETTERS.find((letter) => letter !== 's' && !used.has(letter)) ?? null;
}

/**
 * The alias for a newly scanned destination, or null when the scheme does not name it:
 * an unnamed class, a non-home origin with no alias to inherit a branch from, or a branch
 * and type whose 25 slots are all taken.
 */
export function generateEchoAlias(
	originAlias: string | null | undefined,
	aliases: readonly string[],
	context: EchoContext,
): string | null {
	const type = echoTypeChar(context.targetClassId);
	if (type === null) return null;

	let branch: string;
	if (context.originIsHome) {
		if (context.isHomeStatic) {
			// The static takes the lowest free letter, so the primary static reads `a…s`.
			branch = nextBranchLetter(context.homeBranchLetters);
		} else {
			// Home's statics own the first letters whether or not they are scanned yet, so
			// a wandering hole off home starts past them.
			const reserved = BRANCH_LETTERS.slice(0, Math.max(0, context.homeStaticCount));
			branch = nextBranchLetter([...context.homeBranchLetters, ...reserved]);
		}
	} else {
		branch = (originAlias ?? '').trim().replace(/^\+/, '').charAt(0).toLowerCase();
		if (!branch) return null;
	}

	if (context.originIsHome && context.isHomeStatic) return `${branch}${type}s`;

	const slot = nextSlotLetter(branch, type, aliases);
	return slot === null ? null : `${branch}${type}${slot}`;
}

/**
 * Every class the map can draw, so a test can check the table above has not drifted from
 * the class catalogue. Exported for that test alone.
 */
export function knownWormholeClassIds(): number[] {
	const ids: number[] = [];
	for (let id = 1; id <= 25; id++) {
		if (classMeta(id, null).short !== '?') ids.push(id);
	}
	return ids;
}

/**
 * What this scheme produces: a branch letter, the destination's class, a slot letter. The
 * class is one or two characters (`13` for shattered), and `H`/`L`/`N`/`P`/`T`/`D` stand
 * for k-space, Pochven, Thera and the drifters.
 *
 * Used to tell a name this scheme wrote from one somebody typed, since nothing records
 * which is which. A name we wrote is ours to correct when the hole turns out to lead
 * somewhere else; a name that was meant is not.
 */
const GENERATED_ALIAS = /^[a-z](?:\d{1,2}|[HLNPTD])[a-z]$/;

export function isGeneratedAlias(alias: string | null | undefined): boolean {
	return GENERATED_ALIAS.test((alias ?? '').trim());
}
