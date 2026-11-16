// Placement des info-bulles stylées (`TooltipLayer`) : pur, testé. Côté
// préféré (`data-tip-side`), bascule du côté opposé si la bulle sortirait de
// la fenêtre, puis recadrage dans la fenêtre (marge 4 px).
export type TipSide = 'top' | 'bottom' | 'left' | 'right';

export interface Rect {
	left: number;
	top: number;
	width: number;
	height: number;
}

const GAP = 6;
const MARGIN = 4;
const OPPOSITE: Record<TipSide, TipSide> = {
	top: 'bottom',
	bottom: 'top',
	left: 'right',
	right: 'left'
};

function place(side: TipSide, a: Rect, tip: { width: number; height: number }) {
	const cx = a.left + a.width / 2 - tip.width / 2;
	const cy = a.top + a.height / 2 - tip.height / 2;
	switch (side) {
		case 'top':
			return { left: cx, top: a.top - tip.height - GAP };
		case 'bottom':
			return { left: cx, top: a.top + a.height + GAP };
		case 'left':
			return { left: a.left - tip.width - GAP, top: cy };
		case 'right':
			return { left: a.left + a.width + GAP, top: cy };
	}
}

function fits(
	p: { left: number; top: number },
	tip: { width: number; height: number },
	vw: number,
	vh: number
) {
	return (
		p.left >= MARGIN &&
		p.top >= MARGIN &&
		p.left + tip.width <= vw - MARGIN &&
		p.top + tip.height <= vh - MARGIN
	);
}

/** Position (coordonnées fenêtre) et côté retenu d'une bulle. */
export function tooltipPosition(
	anchor: Rect,
	tip: { width: number; height: number },
	viewport: { width: number; height: number },
	side: TipSide = 'top'
): { left: number; top: number; side: TipSide } {
	let chosen = side;
	let p = place(side, anchor, tip);
	if (!fits(p, tip, viewport.width, viewport.height)) {
		const alt = place(OPPOSITE[side], anchor, tip);
		if (fits(alt, tip, viewport.width, viewport.height)) {
			p = alt;
			chosen = OPPOSITE[side];
		}
	}
	const left = Math.min(Math.max(MARGIN, p.left), viewport.width - tip.width - MARGIN);
	const top = Math.min(Math.max(MARGIN, p.top), viewport.height - tip.height - MARGIN);
	return { left, top, side: chosen };
}
