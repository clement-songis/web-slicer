// Tests de couverture des menus/barres/raccourcis (T061) : `menus.ts` doit
// couvrir exactement les entrées de l'Annexe B §B.3–B.6 et de la barre d'outils
// §B.4. On reparse l'annexe (source de parité) pour détecter tout écart.
import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
	MAIN_TOOLBAR,
	ASSEMBLE_TOOLBAR,
	PLATER_MENU,
	PLATER_SHORTCUTS,
	OBJECTS_LIST_SHORTCUTS,
	GIZMO_SHORTCUTS
} from './menus';

const ANNEXE = readFileSync(
	resolve(
		import.meta.dir,
		'../../../../specs/001-orcaslicer-web-parity/annexes/annexe-b-interface.md'
	),
	'utf8'
);

function section(a: string, b: string): string {
	return ANNEXE.slice(ANNEXE.indexOf(`## ${a}`), ANNEXE.indexOf(`## ${b}`));
}

function subblock(title: string): string {
	const s = ANNEXE.indexOf(`### ${title}`);
	const e = ANNEXE.indexOf('###', s + 3);
	return ANNEXE.slice(s, e === -1 ? undefined : e);
}

/** Libellés du menu contextuel (2e colonne des tables B.3), dédupliqués. */
function menuLabels(): string[] {
	const out: string[] = [];
	for (const line of section('B.3', 'B.4').split('\n')) {
		const m = line.match(/^\|\s*[^|]+\|\s*(.+?)\s*\|$/);
		if (!m) continue;
		const label = m[1];
		if (label === 'Libellé' || /^-+$/.test(label)) continue;
		if (!out.includes(label)) out.push(label);
	}
	return out;
}

/** Touches (1re colonne entre backticks) d'un sous-bloc de raccourcis. */
function shortcutKeys(title: string): string[] {
	const out: string[] = [];
	for (const line of subblock(title).split('\n')) {
		const m = line.match(/^\|\s*`([^`]+)`\s*\|/);
		if (m && !out.includes(m[1])) out.push(m[1]);
	}
	return out;
}

describe('couverture menus.ts ↔ Annexe B', () => {
	test('menu contextuel du plateau (B.3) : tous les libellés couverts', () => {
		const covered = new Set(PLATER_MENU.map((m) => m.label));
		const missing = menuLabels().filter((l) => !covered.has(l));
		expect(missing).toEqual([]);
	});

	test('barre d’outils 3D (B.4) : identifiants attendus présents', () => {
		const ids = new Set([...MAIN_TOOLBAR, ...ASSEMBLE_TOOLBAR].map((b) => b.id));
		for (const id of [
			'add',
			'addplate',
			'orient',
			'arrange',
			'more',
			'fewer',
			'splitobjects',
			'splitvolumes',
			'layersediting',
			'assembly_view'
		]) {
			expect(ids.has(id)).toBe(true);
		}
	});

	test('raccourcis Plater (B.6) : toutes les touches couvertes', () => {
		const covered = new Set(PLATER_SHORTCUTS.map((s) => s.keys));
		const missing = shortcutKeys('Plater').filter((k) => !covered.has(k));
		expect(missing).toEqual([]);
	});

	test('raccourcis Objects List (B.6) : toutes les touches couvertes', () => {
		const covered = new Set(OBJECTS_LIST_SHORTCUTS.map((s) => s.keys));
		const missing = shortcutKeys('Objects List').filter((k) => !covered.has(k));
		expect(missing).toEqual([]);
	});

	test('raccourcis Gizmo (B.6) : toutes les touches couvertes', () => {
		const covered = new Set(GIZMO_SHORTCUTS.map((s) => s.keys));
		const missing = shortcutKeys('Gizmo').filter((k) => !covered.has(k));
		expect(missing).toEqual([]);
	});
});
