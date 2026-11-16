// Garde-fou WCAG AA (passe ergonomie) : les paires jeton-texte / jeton-surface
// du système de design (layout.css) doivent rester ≥ 4.5:1 (texte) ou ≥ 3:1
// (contours de champs, WCAG 1.4.11) dans les deux thèmes. Lit les valeurs
// réelles du CSS : toute régression de palette casse ce test.
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('../../routes/layout.css', import.meta.url), 'utf8');

function block(selector: string): Record<string, string> {
	const start = css.indexOf(selector + ' {');
	if (start < 0) throw new Error('bloc introuvable : ' + selector);
	const body = css.slice(start, css.indexOf('}', start));
	const out: Record<string, string> = {};
	for (const m of body.matchAll(/--ws-([a-z-]+):\s*(#[0-9a-f]{6})/gi)) out[m[1]] = m[2];
	return out;
}

function lum(hex: string): number {
	const n = parseInt(hex.slice(1), 16);
	const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => {
		const s = v / 255;
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
	});
	return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
}

function contrast(a: string, b: string): number {
	const [x, y] = [lum(a), lum(b)];
	return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

const light = block(':root');
const dark = { ...light, ...block(":root[data-theme='dark']") };

const TEXT = ['content', 'content-muted', 'content-subtle', 'primary-text', 'danger-text'];
const SURFACES = ['surface', 'surface-raised'];

describe.each([
	['clair', light],
	['sombre', dark]
] as const)('contrastes WCAG AA — thème %s', (_name, tok) => {
	it.each(TEXT)('texte %s ≥ 4.5:1 sur les surfaces', (fg) => {
		for (const bg of SURFACES) expect(contrast(tok[fg], tok[bg])).toBeGreaterThanOrEqual(4.5);
	});

	it.each(['success-text', 'warning-text'])('%s ≥ 4.5:1 sur surface surélevée', (fg) => {
		expect(contrast(tok[fg], tok['surface-raised'])).toBeGreaterThanOrEqual(4.5);
	});

	it.each(['primary', 'primary-hover', 'success', 'warning', 'danger'])(
		'remplissage %s ≥ 4.5:1 sous texte blanc',
		(fill) => {
			expect(contrast(tok[fill], '#ffffff')).toBeGreaterThanOrEqual(4.5);
		}
	);

	it('bulle inversée (.ws-tooltip, étiquettes 3D) : texte ≥ 4.5:1', () => {
		expect(contrast(tok['on-inverse'], tok['inverse'])).toBeGreaterThanOrEqual(4.5);
	});

	it.each(['success', 'warning', 'danger'])('bandeau %s : texte ≥ 4.5:1 sur fond soft', (tone) => {
		expect(contrast(tok[tone + '-content'], tok[tone + '-soft'])).toBeGreaterThanOrEqual(4.5);
	});

	it('contours de champs et anneau de focus ≥ 3:1', () => {
		expect(contrast(tok['border-strong'], tok['surface-raised'])).toBeGreaterThanOrEqual(3);
		expect(contrast(tok['accent'], tok['surface-raised'])).toBeGreaterThanOrEqual(3);
	});
});
