// Tests de l'orchestrateur d'import (T089) : classification des formats et
// cycle de vie d'un import (upload → conversion moteur → prêt / échec).
import { describe, expect, it } from 'vitest';
import {
	findByModel,
	importExt,
	importLabel,
	isAccepted,
	isPending,
	markConverted,
	markFailed,
	markUploaded,
	startImport
} from './imports';

describe('imports', () => {
	it('classe les formats acceptés à l’upload', () => {
		expect(importExt('Part.STL')).toBe('stl');
		expect(isAccepted('part.stl')).toBe(true);
		expect(isAccepted('part.obj')).toBe(true);
		expect(isAccepted('part.3mf')).toBe(true);
		expect(isAccepted('part.step')).toBe(true);
		expect(isAccepted('part.stp')).toBe(true);
		// Jeu OrcaSlicer étendu (T091).
		expect(isAccepted('part.oltp')).toBe(true);
		expect(isAccepted('part.amf')).toBe(true);
		expect(isAccepted('part.svg')).toBe(true);
		expect(isAccepted('part.drc')).toBe(true);
		// Hors jeu v1 (exclusions.md) et formats non-modèles.
		expect(isAccepted('part.zip')).toBe(false);
		expect(isAccepted('part.ply')).toBe(false);
		expect(isAccepted('part.gcode')).toBe(false);
	});

	it('démarre un import en upload', () => {
		const item = startImport('obj-1', 'part.stl');
		expect(item.status).toBe('uploading');
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

	it('expose l’état en cours et un libellé de placeholder/badge (T127)', () => {
		const uploading = startImport('o1', 'part.stl');
		expect(isPending(uploading)).toBe(true);
		expect(importLabel(uploading)).toBe('import en cours…');

		const converting = markUploaded(uploading, 'm1', true);
		expect(isPending(converting)).toBe(true);
		expect(importLabel(converting)).toBe('conversion en cours…');

		const ready = markConverted(converting);
		expect(isPending(ready)).toBe(false);
		expect(importLabel(ready)).toBe('prêt');

		const failed = markFailed(converting, 'échec de la conversion du modèle');
		expect(isPending(failed)).toBe(false);
		expect(importLabel(failed)).toBe('échec de la conversion du modèle');
	});
});
