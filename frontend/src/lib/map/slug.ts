// A map's address reads as its name with the id on the end, `/maps/home-chain-42`, the way
// the old site wrote it. Only the id counts: the name part is never stored or looked up,
// so a renamed map's old links, and a bare `/maps/42`, still lead to it.

/** The map's name as an address segment, with its id last. */
export function mapSlug(map: { id: number; name: string }): string {
	const name = map.name
		.normalize('NFKD')
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '');
	return name ? `${name}-${map.id}` : `${map.id}`;
}

/** `/maps/<slug>`, the map's page. */
export function mapPath(map: { id: number; name: string }): string {
	return `/maps/${mapSlug(map)}`;
}

/** The id at the end of a slug, or NaN when there is none. */
export function mapIdFromSlug(slug: string): number {
	const last = slug.slice(slug.lastIndexOf('-') + 1);
	return /^\d+$/.test(last) ? Number(last) : NaN;
}
