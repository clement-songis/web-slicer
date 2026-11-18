// Tests du réducteur de session de tranchage (T088) : transitions, progression,
// bascule vers l'aperçu, isolation et gel des erreurs.
import { describe, expect, it } from 'vitest';
import type { ServerEvent } from '../api/types';
import {
	applyJobEvent,
	engineWarnings,
	friendlySliceError,
	prepareSession,
	resetSession,
	sliceFailed,
	startSlicing
} from './session';

const updated = (
	id: string,
	progress: number,
	phase = 'slice',
	status = 'running'
): ServerEvent => ({
	event: 'job.updated',
	id,
	status,
	progress,
	phase
});

const finished = (id: string, gcode_id?: string): ServerEvent => ({
	event: 'job.finished',
	id,
	gcode_id,
	stats: null
});

describe('slice session', () => {
	it('démarre en préparation', () => {
		const s = prepareSession();
		expect(s.phase).toBe('prepare');
		expect(s.jobIds).toEqual([]);
		expect(s.gcodeId).toBeNull();
	});

	it('passe en tranchage en conservant les avertissements', () => {
		const s = startSlicing(['j1'], [{ key: 'k', message: 'attention' }]);
		expect(s.phase).toBe('slicing');
		expect(s.jobIds).toEqual(['j1']);
		expect(s.warnings).toHaveLength(1);
	});

	it('suit la progression du job suivi', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, updated('j1', 0.4, 'export'));
		expect(s.progress).toBeCloseTo(0.4);
		expect(s.jobPhase).toBe('export');
		expect(s.phase).toBe('slicing');
	});

	it('ignore les événements des jobs étrangers (isolation)', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, updated('autre', 0.9));
		expect(s.progress).toBe(0);
		expect(s.phase).toBe('slicing');
	});

	it('bascule en aperçu au job terminé avec G-code', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, finished('j1', 'g-123'));
		expect(s.phase).toBe('preview');
		expect(s.progress).toBe(1);
		expect(s.gcodeId).toBe('g-123');
	});

	it('erreur si le job échoue, puis fige (événements suivants sans effet)', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, updated('j1', 0.5, 'slice', 'failed'));
		expect(s.phase).toBe('error');
		expect(s.error).toBe('échec du tranchage');
		// Gel : un job.finished tardif ne ressuscite pas l'aperçu.
		s = applyJobEvent(s, finished('j1', 'g-123'));
		expect(s.phase).toBe('error');
		expect(s.gcodeId).toBeNull();
	});

	it('erreur si terminé sans G-code', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, finished('j1', undefined));
		expect(s.phase).toBe('error');
	});

	it('clampe une progression hors bornes', () => {
		let s = startSlicing(['j1']);
		s = applyJobEvent(s, updated('j1', 5));
		expect(s.progress).toBe(1);
	});

	it('sliceFailed et resetSession', () => {
		expect(sliceFailed('boom').phase).toBe('error');
		expect(sliceFailed('boom').error).toBe('boom');
		expect(resetSession().phase).toBe('prepare');
	});
});

describe('échec moteur générique : avertissements du moteur remontés', () => {
	const raw =
		'worker sorti en code 1 — stderr :  — stdout : P 0.05 Slicing mesh\nP 0.5 Checking support necessity\nP -0.01 It seems object big600k.stl has floating regions. Please re-orient the object or enable support generation.\nP 0.7 Generating skirt & brim\nE engine_crashed Errors\n';
	it('« Errors » seul devient explicite', () => {
		expect(engineWarnings(raw)).toEqual([
			'« big600k.stl » a des zones flottantes (rien en dessous) : réorientez-le, posez-le sur le plateau ou activez les supports.'
		]);
		expect(friendlySliceError(raw)).toMatch(
			/^Le moteur a échoué \(Errors\)\. « big600k\.stl » a des zones flottantes/
		);
	});
	it('message déjà précis inchangé', () => {
		expect(friendlySliceError('E invalid_config Hauteur de couche trop grande')).toBe(
			'Hauteur de couche trop grande'
		);
	});
});
