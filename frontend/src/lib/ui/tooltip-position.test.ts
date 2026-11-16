import { describe, expect, it } from 'vitest';
import { tooltipPosition } from './tooltip-position';

const vp = { width: 1000, height: 800 };
const tip = { width: 120, height: 24 };

describe('tooltipPosition', () => {
	it('côté préféré quand la bulle tient', () => {
		const p = tooltipPosition({ left: 500, top: 400, width: 36, height: 36 }, tip, vp, 'left');
		expect(p.side).toBe('left');
		expect(p.left + tip.width).toBeLessThanOrEqual(500);
	});
	it('bascule du côté opposé au bord de la fenêtre (rail à droite → gauche)', () => {
		const p = tooltipPosition({ left: 960, top: 100, width: 36, height: 36 }, tip, vp, 'right');
		expect(p.side).toBe('left');
	});
	it('recadre dans la fenêtre', () => {
		const p = tooltipPosition({ left: 0, top: 0, width: 20, height: 20 }, tip, vp, 'top');
		expect(p.left).toBeGreaterThanOrEqual(4);
		expect(p.top).toBeGreaterThanOrEqual(4);
	});
});
