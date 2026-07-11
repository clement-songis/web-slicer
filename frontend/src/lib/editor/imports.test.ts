// Tests de l'orchestrateur d'import (T089) : classification des formats et
// cycle de vie d'un import (aperçu → upload → conversion / échec).
import { describe, expect, it } from 'bun:test';
import {
	findByModel,
	importExt,
	isAccepted,
	isPreviewable,
	markConverted,
	markFailed,
	markUploaded,
	startImport
} from './imports';

describe('imports', () => {
	it('classe les formats acceptés et prévisualisables', () => {
		expect(importExt('Part.STL')).toBe('stl');
		expect(isAccepted('part.stl')).toBe(true);
		expect(isAccepted('part.obj')).toBe(true);
		expect(isAccepted('part.3mf')).toBe(true);
		expect(isAccepted('part.step')).toBe(true);
		expect(isAccepted('part.stp')).toBe(true);
		expect(isAccepted('part.gcode')).toBe(false);
		// STEP est accepté mais non prévisualisable côté client (conversion moteur).
		expect(isPreviewable('part.stl')).toBe(true);
		expect(isPreviewable('part.step')).toBe(false);
	});

	it('démarre un import en aperçu', () => {
		const item = startImport('obj-1', 'part.stl');
		expect(item.status).toBe('previewing');
		expect(item.objectId).toBe('obj-1');
		expect(item.modelId).toBeNull();
	});

	it('passe en prêt si le maillage est déjà là, sinon en conversion', () => {
		const ready = markUploaded(startImport('o1', 'part.stl'), 'm1', false);
		expect(ready.status).toBe('ready');
		expect(ready.modelId).toBe('m1');

		const converting = markUploaded(startImport('o2', 'part.step'), 'm2', true);
		expect(converting.status).toBe('converting');
	});

	it('résout la conversion et retrouve l’item par modèle', () => {
		const items = [
			markUploaded(startImport('o1', 'a.stl'), 'm1', false),
			markUploaded(startImport('o2', 'b.step'), 'm2', true)
		];
		const target = findByModel(items, 'm2');
		expect(target?.objectId).toBe('o2');
		expect(markConverted(target!).status).toBe('ready');
		expect(findByModel(items, 'absent')).toBeUndefined();
	});

	it('fige une erreur', () => {
		const failed = markFailed(startImport('o1', 'part.stl'), 'STL illisible');
		expect(failed.status).toBe('failed');
		expect(failed.error).toBe('STL illisible');
	});
});
