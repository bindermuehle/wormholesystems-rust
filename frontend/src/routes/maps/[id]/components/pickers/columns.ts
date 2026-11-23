// Shared layout for `SystemRow` lists: class, name, region, holder cell (sovereignty or
// effect).
//
// The list owns the tracks and every descendant down to the cells is a subgrid
// (`col-span-full grid grid-cols-subgrid`). A grid declared per row would size its columns
// from its own content, so rows in the same list would not line up. Tracks are spelled out
// as complete literals (never concatenated) so Tailwind sees them at build time.
// Command-driven lists need one extra trailing track for the check indicator `Command.Item`
// appends, or the row wraps onto a second grid line and doubles in height.

/** The four SystemRow tracks plus Command's check indicator. */
export const SYSTEM_LIST =
	'grid grid-cols-[min-content_minmax(0,1fr)_minmax(0,0.8fr)_min-content_min-content] items-center gap-x-2';
/** As SYSTEM_LIST, with a hint/badge track before the indicator (the command palette). */
export const SYSTEM_LIST_HINT =
	'grid grid-cols-[min-content_minmax(0,1fr)_minmax(0,0.8fr)_min-content_minmax(0,0.7fr)_min-content] items-center gap-x-2';
/**
 * As SYSTEM_LIST, with a trailing auto track for row actions (watchlist, find). The track
 * before it holds the jump count, and the route strip once the panel is wide enough to draw
 * one: from there the name and region take what they need and the route gets the rest.
 * The class track is a fixed width there: a header spanning it and the name would otherwise
 * be shared out between the two now that neither is flexible, and push the names away
 * from their class letters.
 */
export const SYSTEM_LIST_ACTIONS =
	'grid grid-cols-[min-content_minmax(0,1fr)_minmax(0,0.8fr)_min-content_min-content_auto] items-center gap-x-2 @[36rem]/routes:grid-cols-[1.5rem_minmax(0,auto)_minmax(0,auto)_min-content_minmax(min-content,1fr)_auto]';

/** A full row inside one of the lists above. */
export const SYSTEM_ROW = 'col-span-full grid grid-cols-subgrid items-center gap-x-2';
/** The four SystemRow cells inside a row (the remaining tracks are the list's own). */
export const SYSTEM_CELLS_4 = 'col-span-4 grid grid-cols-subgrid items-center gap-x-2';
/** The five cells when the list carries the hint track. */
export const SYSTEM_CELLS_5 = 'col-span-5 grid grid-cols-subgrid items-center gap-x-2';
