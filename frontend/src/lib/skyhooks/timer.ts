// When a skyhook's theft window is, in the terms the card talks about. Everything here is
// pure and takes `now` as an argument so the panel and the row cannot disagree, and so the
// tests do not have to wait two hours for a real window.

import type { Skyhook } from '$lib/api/types/Skyhook';

export type SkyhookStatus =
	/** The window has not opened yet. */
	| 'upcoming'
	/** Open, with time to spare. */
	| 'open'
	/** Open, but not for much longer. */
	| 'closing'
	/** Already over; the row should not be shown at all. */
	| 'closed';

/** How little is left before "open" becomes "hurry". */
export const CLOSING_SOON_MS = 15 * 60 * 1000;

export interface SkyhookTiming {
	status: SkyhookStatus;
	/** Milliseconds to the close while open, to the open while upcoming, since the close once over. */
	untilMs: number;
}

export function timing(skyhook: Skyhook, now: Date): SkyhookTiming {
	const from = new Date(skyhook.vulnerable_from).getTime();
	const until = new Date(skyhook.vulnerable_until).getTime();
	const at = now.getTime();

	if (at >= until) return { status: 'closed', untilMs: at - until };
	if (at < from) return { status: 'upcoming', untilMs: from - at };
	const left = until - at;
	return { status: left < CLOSING_SOON_MS ? 'closing' : 'open', untilMs: left };
}

/** `<1m`, `47m`, `2h 05m`, `1d 04h`. Coarse on purpose, a ticking second count is noise. */
export function formatDuration(ms: number): string {
	const minutes = Math.floor(Math.abs(ms) / 60_000);
	if (minutes < 1) return '<1m';
	if (minutes < 60) return `${minutes}m`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) {
		const rest = minutes % 60;
		return rest > 0 ? `${hours}h ${String(rest).padStart(2, '0')}m` : `${hours}h`;
	}
	const days = Math.floor(hours / 24);
	const rest = hours % 24;
	return rest > 0 ? `${days}d ${String(rest).padStart(2, '0')}h` : `${days}d`;
}

/** `12:46 – 14:46 UTC`, which is how a timer gets written down and shared. */
export function formatWindow(skyhook: Skyhook): string {
	const at = (iso: string) => {
		const d = new Date(iso);
		return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`;
	};
	return `${at(skyhook.vulnerable_from)} – ${at(skyhook.vulnerable_until)} UTC`;
}

/** The sentence above the window in the tooltip. */
export function describe(t: SkyhookTiming): string {
	const spell = formatDuration(t.untilMs);
	if (t.status === 'closed') return `Closed ${spell} ago`;
	if (t.status === 'upcoming') return `Raidable in ${spell}`;
	return `Raidable for ${spell}`;
}

// The mark at the start of a row. Its shape says the status (see the card), so it holds
// still: the timer beside it is what moves.
const MARK = {
	upcoming: 'text-amber-400',
	open: 'text-emerald-400',
	closing: 'text-red-400',
	closed: 'text-muted-foreground/40',
} satisfies Record<SkyhookStatus, string>;

const TEXT = {
	upcoming: 'text-amber-400',
	open: 'text-emerald-400',
	closing: 'text-red-400 animate-pulse',
	closed: 'text-muted-foreground/60',
} satisfies Record<SkyhookStatus, string>;

export function statusMark(status: SkyhookStatus): string {
	return MARK[status];
}

export function statusText(status: SkyhookStatus): string {
	return TEXT[status];
}
