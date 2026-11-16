// Tests des constructeurs de classes UI (T094). On vérifie que chaque variante
// sélectionne bien les jetons attendus (et jamais une échelle de gris brute).
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import {
	cx,
	buttonClasses,
	iconButtonClasses,
	fieldClasses,
	bannerClasses,
	segmentClasses,
	tabClasses,
	toolClasses
} from './styles';

describe('cx', () => {
	it('joint en ignorant les fragments vides', () => {
		expect(cx('a', false, null, undefined, 'b')).toBe('a b');
		expect(cx()).toBe('');
	});
});

// Les recettes .ws-* vivent dans layout.css : on vérifie à la fois le
// constructeur (bonnes classes) et la recette CSS (bons jetons).
const css = readFileSync(new URL('../../routes/layout.css', import.meta.url), 'utf8');
function recipe(cls: string): string {
	const m = css.match(new RegExp('\\.' + cls + ' \\{\\s*@apply ([^;]+);'));
	if (!m) throw new Error('recette introuvable : ' + cls);
	return m[1];
}

describe('buttonClasses', () => {
	it('primary utilise les jetons primaires', () => {
		expect(buttonClasses('primary')).toContain('ws-btn-primary');
		const r = recipe('ws-btn-primary');
		expect(r).toContain('bg-primary');
		expect(r).toContain('text-primary-content');
		expect(r).toContain('hover:bg-primary-hover');
	});

	it('secondary/ghost/danger ciblent les bons jetons', () => {
		expect(recipe('ws-btn-secondary')).toContain('bg-surface-raised');
		expect(recipe('ws-btn-ghost')).toContain('hover:bg-overlay');
		expect(recipe('ws-btn-danger')).toContain('bg-danger');
	});

	it('la taille pilote la hauteur (28 / 32 / 40 px)', () => {
		expect(buttonClasses('primary', 'sm')).toContain('ws-btn-sm');
		expect(buttonClasses('primary', 'lg')).toContain('ws-btn-lg');
		expect(recipe('ws-btn')).toContain('h-8');
		expect(recipe('ws-btn-sm')).toContain('h-7');
		expect(recipe('ws-btn-lg')).toContain('h-10');
	});

	it('primary/md par défaut, avec anneau de focus accessible', () => {
		const c = buttonClasses();
		expect(c).toContain('ws-btn ');
		expect(c).toContain('ws-btn-primary');
		expect(c).not.toContain('ws-btn-sm');
		expect(c).toContain('focus-visible:ring-accent');
	});

	it('aucune recette n’émet de couleur brute', () => {
		for (const m of css.matchAll(/\.ws-[a-z-]+ \{\s*@apply ([^;]+);/g)) {
			expect(m[1]).not.toMatch(/\b(bg|text|border)-(gray|slate|zinc|white|black|blue|red|green)\b/);
		}
	});
});

describe('iconButtonClasses', () => {
	it('bouton carré aligné sur la hauteur des boutons, palette de variante', () => {
		expect(iconButtonClasses('ghost', 'sm')).toContain('ws-icon-btn-sm');
		expect(iconButtonClasses('secondary', 'md')).toContain('ws-btn-secondary');
		expect(recipe('ws-icon-btn')).toContain('h-8 w-8');
	});
});

describe('fieldClasses', () => {
	it('champ : hauteur alignée sur les boutons, bordure ≥ 3:1, chiffres tabulaires', () => {
		expect(fieldClasses()).toBe('ws-field');
		expect(fieldClasses('sm')).toBe('ws-field ws-field-sm');
		const r = recipe('ws-field');
		expect(r).toContain('h-8');
		expect(r).toContain('border-border-strong');
		expect(r).toContain('tabular-nums');
	});
});

describe('bannerClasses', () => {
	it('chaque tonalité utilise sa paire soft/content', () => {
		expect(bannerClasses('success')).toContain('bg-success-soft');
		expect(bannerClasses('success')).toContain('text-success-content');
		expect(bannerClasses('warning')).toContain('bg-warning-soft');
		expect(bannerClasses('danger')).toContain('text-danger-content');
	});

	it('info est neutre bordé, défaut = info', () => {
		expect(bannerClasses('info')).toContain('bg-surface-sunken');
		expect(bannerClasses()).toContain('bg-surface-sunken');
	});
});

describe('segmentClasses / tabClasses / toolClasses', () => {
	it('actif = primaire, inactif = discret', () => {
		expect(segmentClasses(true)).toContain('ws-seg-on');
		expect(recipe('ws-seg-on')).toContain('bg-primary');
		expect(recipe('ws-seg-item')).toContain('text-content-muted');
		expect(tabClasses(true)).toContain('ws-tab-on');
		expect(recipe('ws-tab-on')).toContain('border-primary');
		expect(recipe('ws-tab')).toContain('text-content-muted');
		expect(toolClasses(true)).toContain('ws-tool-on');
		expect(toolClasses(false, 'sm')).toContain('ws-tool-sm');
		expect(recipe('ws-tool-on')).toContain('bg-primary');
	});

	it('onglet inactif garde une bordure transparente (pas de saut de mise en page)', () => {
		expect(recipe('ws-tab')).toContain('border-b-2');
		expect(recipe('ws-tab')).toContain('border-transparent');
		expect(tabClasses(false)).not.toContain('ws-tab-on');
	});

	it('segments et onglets exposent un focus clavier visible', () => {
		expect(recipe('ws-seg-item')).toContain('focus-visible:ring-accent');
		expect(recipe('ws-tab')).toContain('focus-visible:ring-accent');
	});
});

describe('alignement des boutons', () => {
	it('hauteur fixe par taille, identique bouton/bouton-icône', () => {
		expect(recipe('ws-btn-sm')).toContain('h-7');
		expect(recipe('ws-icon-btn-sm')).toContain('h-7 w-7');
		expect(recipe('ws-btn')).toContain('h-8');
		expect(recipe('ws-icon-btn')).toContain('h-8 w-8');
	});

	it('désactivé : curseur interdit et opacité réduite', () => {
		expect(recipe('ws-btn')).toContain('disabled:cursor-not-allowed');
		expect(recipe('ws-btn')).toContain('disabled:opacity-50');
	});
});
