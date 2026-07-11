// Tests des constructeurs de classes UI (T094). On vérifie que chaque variante
// sélectionne bien les jetons attendus (et jamais une échelle de gris brute).
import { describe, expect, it } from 'bun:test';
import {
	cx,
	buttonClasses,
	iconButtonClasses,
	bannerClasses,
	segmentClasses,
	tabClasses
} from './styles';

describe('cx', () => {
	it('joint en ignorant les fragments vides', () => {
		expect(cx('a', false, null, undefined, 'b')).toBe('a b');
		expect(cx()).toBe('');
	});
});

describe('buttonClasses', () => {
	it('primary utilise les jetons primaires', () => {
		const c = buttonClasses('primary');
		expect(c).toContain('bg-primary');
		expect(c).toContain('text-primary-content');
		expect(c).toContain('hover:bg-primary-hover');
	});

	it('secondary/ghost/danger ciblent les bons jetons', () => {
		expect(buttonClasses('secondary')).toContain('bg-surface-raised');
		expect(buttonClasses('ghost')).toContain('hover:bg-overlay');
		expect(buttonClasses('danger')).toContain('bg-danger');
	});

	it('la taille pilote le rembourrage', () => {
		expect(buttonClasses('primary', 'sm')).toContain('px-2.5');
		expect(buttonClasses('primary', 'md')).toContain('px-4');
	});

	it('primary/md par défaut, avec anneau de focus accessible', () => {
		const c = buttonClasses();
		expect(c).toContain('bg-primary');
		expect(c).toContain('px-4');
		expect(c).toContain('focus-visible:ring-accent');
	});

	it('n’émet aucune échelle de gris brute', () => {
		for (const v of ['primary', 'secondary', 'ghost', 'danger'] as const) {
			expect(buttonClasses(v)).not.toMatch(/\bbg-gray-/);
		}
	});
});

describe('iconButtonClasses', () => {
	it('applique un padding carré et la palette de variante', () => {
		expect(iconButtonClasses('ghost', 'sm')).toContain('p-1');
		expect(iconButtonClasses('secondary', 'md')).toContain('p-1.5');
		expect(iconButtonClasses('secondary')).toContain('bg-surface-raised');
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

describe('segmentClasses / tabClasses', () => {
	it('actif = primaire, inactif = discret', () => {
		expect(segmentClasses(true)).toContain('bg-primary');
		expect(segmentClasses(false)).toContain('text-content-muted');
		expect(tabClasses(true)).toContain('border-primary');
		expect(tabClasses(false)).toContain('text-content-muted');
	});
});
