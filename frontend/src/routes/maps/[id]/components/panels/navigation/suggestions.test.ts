import { describe, expect, it } from 'vitest';

import type { CharacterRef } from '$lib/api/types/CharacterRef';
import { pilotLocations } from './suggestions';

function pilot(
	name: string,
	system: number | null,
	over: Partial<CharacterRef> = {},
): CharacterRef {
	return {
		character_id: name.length,
		name,
		is_active: false,
		is_preferred: false,
		online: true,
		solar_system_id: system,
		needs_login: false,
		missing_scopes: [],
		desktop_map_id: null,
		...over,
	};
}

describe('pilotLocations', () => {
	it('offers every online pilot, not only the first', () => {
		const located = pilotLocations([pilot('A', 1), pilot('B', 2)]);
		expect([...located.keys()]).toEqual([1, 2]);
	});

	it('puts the active pilot first and shares a system between those in it', () => {
		const located = pilotLocations([
			pilot('A', 1),
			pilot('B', 2, { is_active: true }),
			pilot('C', 1),
		]);
		expect([...located]).toEqual([
			[2, ['B']],
			[1, ['A', 'C']],
		]);
	});

	it('leaves out pilots who are offline or nowhere', () => {
		const located = pilotLocations([pilot('A', 1, { online: false }), pilot('B', null)]);
		expect(located.size).toBe(0);
	});
});
