import { describe, expect, it } from 'vitest';

import { mapIdFromSlug, mapPath, mapSlug } from './slug';

describe('mapSlug', () => {
	it('puts the id after the name', () => {
		expect(mapSlug({ id: 42, name: 'Home Chain' })).toBe('home-chain-42');
	});

	it('collapses punctuation and drops accents', () => {
		expect(mapSlug({ id: 7, name: '  Café / C5 -- Static!  ' })).toBe('cafe-c5-static-7');
	});

	it('is the id alone when the name leaves nothing', () => {
		expect(mapSlug({ id: 3, name: '???' })).toBe('3');
	});

	it('is a path under /maps', () => {
		expect(mapPath({ id: 42, name: 'Home' })).toBe('/maps/home-42');
	});
});

describe('mapIdFromSlug', () => {
	it('reads the id off the end', () => {
		expect(mapIdFromSlug('home-chain-42')).toBe(42);
	});

	it('takes a bare id', () => {
		expect(mapIdFromSlug('42')).toBe(42);
	});

	it('ignores digits in the name', () => {
		expect(mapIdFromSlug('c5-c5-12')).toBe(12);
	});

	it('is NaN without an id', () => {
		expect(mapIdFromSlug('home-chain')).toBeNaN();
		expect(mapIdFromSlug('home-')).toBeNaN();
		expect(mapIdFromSlug('')).toBeNaN();
	});
});
