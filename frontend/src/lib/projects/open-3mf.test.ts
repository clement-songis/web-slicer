import { describe, expect, it } from 'vitest';

// La suite tourne sous l'environnement node (sans DOM) : mini-shim de
// sessionStorage pour les allers-retours du rapport d'import.
if (!('sessionStorage' in globalThis)) {
	const store = new Map<string, string>();
	Object.defineProperty(globalThis, 'sessionStorage', {
		value: {
			getItem: (k: string) => store.get(k) ?? null,
			setItem: (k: string, v: string) => void store.set(k, v),
			removeItem: (k: string) => void store.delete(k),
			clear: () => store.clear(),
			key: (i: number) => [...store.keys()][i] ?? null,
			get length() {
				return store.size;
			}
		}
	});
}
import {
	behaviourFromChoice,
	decideLoadMode,
	isProjectArchive,
	stashImportReport,
	takeImportReport,
	warningContent
} from './open-3mf';

describe('ouverture d’un 3MF (lot 34)', () => {
	it('seuls les .3mf sont des projets', () => {
		expect(isProjectArchive('cube.3mf')).toBe(true);
		expect(isProjectArchive('CUBE.3MF')).toBe(true);
		expect(isProjectArchive('cube.stl')).toBe(false);
		expect(isProjectArchive('projet.3mf.zip')).toBe(false);
	});

	it('determine_load_type : comportement → mode (Orca)', () => {
		// load_all → projet sans question.
		expect(decideLoadMode('load_all', true)).toBe('project');
		expect(decideLoadMode('load_all', false)).toBe('project');
		// load_geometry_only → géométrie sans question.
		expect(decideLoadMode('load_geometry_only', true)).toBe('geometry');
		// always_ask → toujours la boîte.
		expect(decideLoadMode('always_ask', false)).toBe('ask');
		// ask_when_relevant : demande seulement s'il y a un travail en cours
		// (Orca : plateau non vide → override ALWAYS_ASK).
		expect(decideLoadMode('ask_when_relevant', true)).toBe('ask');
		expect(decideLoadMode('ask_when_relevant', false)).toBe('project');
	});

	it('choix mémorisé : projet → load_all, géométrie → load_geometry_only', () => {
		expect(behaviourFromChoice('project')).toBe('load_all');
		expect(behaviourFromChoice('geometry')).toBe('load_geometry_only');
	});

	it('avertissements : titres et textes des boîtes d’Orca', () => {
		const modified = warningContent({ kind: 'modified_gcodes', items: ['machine_start_gcode'] });
		expect(modified.title).toBe('G-code modifié');
		expect(modified.body).toContain('G-code modifié');
		const custom = warningContent({ kind: 'customized_presets', items: ['Mon preset'] });
		expect(custom.title).toBe('Presets personnalisés');
		expect(custom.body).toContain('presets filament ou imprimante personnalisés');
	});

	it('rapport d’import passé par sessionStorage (une seule lecture)', () => {
		const report = {
			load: 'project' as const,
			gcode_warning: { kind: 'modified_gcodes', items: ['machine_start_gcode'] },
			has_metadata: true
		};
		stashImportReport('p1', report);
		expect(takeImportReport('p1')).toEqual(report);
		// Consommé : la seconde lecture ne rejoue pas l'avertissement.
		expect(takeImportReport('p1')).toBeNull();
		expect(takeImportReport('inconnu')).toBeNull();
	});

	it('rapport illisible ignoré', () => {
		sessionStorage.setItem('web-slicer.import-report:p2', '{pas du json');
		expect(takeImportReport('p2')).toBeNull();
	});
});
