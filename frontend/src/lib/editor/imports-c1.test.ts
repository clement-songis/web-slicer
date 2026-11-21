import { describe, expect, it } from 'vitest';
import {
	clearWaiting,
	importLabel,
	isConnectionError,
	isPending,
	markUploaded,
	markWaiting,
	startImport
} from './imports';

describe('import interrompu par la perte de connexion (audit 42 C1)', () => {
	it('erreurs de liaison reconnues, pas les erreurs du fichier', () => {
		expect(isConnectionError(new TypeError('Failed to fetch'))).toBe(true);
		expect(isConnectionError({ status: 0, code: 'network' })).toBe(true);
		expect(isConnectionError({ status: 503, code: 'error' })).toBe(true);
		expect(isConnectionError({ status: 422, code: 'conversion_failed' })).toBe(false);
		expect(isConnectionError(new Error('x'), true)).toBe(true);
	});
	it('en attente : toujours « en cours » (sauvegarde bloquée), libellé dédié', () => {
		const uploaded = markWaiting(markUploaded(startImport('o', 'big.stl'), 'm1', false));
		expect(uploaded.status).toBe('converting');
		expect(isPending(uploaded)).toBe(true);
		expect(importLabel(uploaded)).toBe('waiting for the connection…');
		const upload = markWaiting(startImport('o2', 'x.stl'));
		expect(upload.status).toBe('uploading');
		expect(clearWaiting(uploaded).waiting).toBeUndefined();
	});
});
