// Audit 49 PERF49-a — raison de blocage du bouton Trancher dérivée de l'état
// des imports (événements), pas d'un minuteur : à l'ouverture d'un 3MF, des
// objets connus (imports prêts) dont les maillages ne sont pas encore rendus
// ne doivent plus s'annoncer « Ajoutez un objet à trancher ».
import { describe, expect, test } from 'vitest';
import { sliceBlockerReason } from './slice-blocker';

const ready = {
	objectCount: 1,
	outsideCount: 0,
	importPending: false,
	hasProcess: true,
	filamentCount: 1,
	meshesLoading: 0
};

describe('PERF49-a — sliceBlockerReason', () => {
	test('tranchable : aucune raison', () => {
		expect(sliceBlockerReason(ready)).toBeNull();
	});

	test('ouverture de 3MF : maillages attendus → chargement, pas « ajoutez un objet »', () => {
		expect(sliceBlockerReason({ ...ready, objectCount: 0, meshesLoading: 3 })).toBe(
			'loading-objects'
		);
	});

	test('premier import en vol (téléversement/conversion) → chargement', () => {
		expect(sliceBlockerReason({ ...ready, objectCount: 0, importPending: true })).toBe(
			'loading-objects'
		);
	});

	test('scène réellement vide → « ajoutez un objet »', () => {
		expect(sliceBlockerReason({ ...ready, objectCount: 0 })).toBe('add-object');
	});

	test('l\u2019activation suit les maillages : le dernier atterrit → plus de raison', () => {
		let s = { ...ready, objectCount: 0, meshesLoading: 2 };
		expect(sliceBlockerReason(s)).toBe('loading-objects');
		s = { ...s, objectCount: 1, meshesLoading: 1 };
		expect(sliceBlockerReason(s)).toBeNull();
		s = { ...s, objectCount: 2, meshesLoading: 0 };
		expect(sliceBlockerReason(s)).toBeNull();
	});

	test('raison prioritaire inchangée dès qu’un objet est rendu (hors plateau, profils)', () => {
		expect(sliceBlockerReason({ ...ready, outsideCount: 1 })).toBe('outside');
		expect(sliceBlockerReason({ ...ready, hasProcess: false })).toBe('process');
		expect(sliceBlockerReason({ ...ready, filamentCount: 0 })).toBe('filament');
		// Objet rendu + autre import en vol : bloqué « Import en cours… » (comportement
		// d’avant préservé — canSliceNow exige qu’aucun import ne soit en attente).
		expect(sliceBlockerReason({ ...ready, importPending: true, meshesLoading: 1 })).toBe(
			'importing'
		);
	});
});
