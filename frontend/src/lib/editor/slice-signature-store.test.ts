import { describe, expect, it } from 'vitest';
import { ApiError } from '$lib/api/client';
import {
	hasRememberedSignature,
	localSignatureVerdict,
	serverJobFreshness,
	shouldRestore,
	rememberSliceSignature,
	restorableResult,
	serverSliceFreshness,
	signatureHash
} from './slice-signature-store';

function memory() {
	const data = new Map<string, string>();
	return {
		data,
		getItem: (k: string) => data.get(k) ?? null,
		setItem: (k: string, v: string) => void data.set(k, v)
	};
}

describe('slice-signature-store (audit 41 V1)', () => {
	it('empreinte stable et discriminante', () => {
		expect(signatureHash('abc')).toBe(signatureHash('abc'));
		expect(signatureHash('abc')).not.toBe(signatureHash('abd'));
		expect(signatureHash('x')).toMatch(/^[0-9a-f]{16}$/);
	});

	it('restaure seulement si la signature du plateau est inchangée', () => {
		const s = memory();
		rememberSliceSignature('p1', 'g1', '{"wall_loops":3}', s);
		expect(restorableResult('p1', 'g1', '{"wall_loops":3}', s)).toBe(true);
		// wall_loops 3 → 5 enregistré : G-code périmé, pas de restauration.
		expect(restorableResult('p1', 'g1', '{"wall_loops":5}', s)).toBe(false);
		// G-code inconnu (autre navigateur) ou autre projet : pas de restauration.
		expect(restorableResult('p1', 'g2', '{"wall_loops":3}', s)).toBe(false);
		expect(restorableResult('p2', 'g1', '{"wall_loops":3}', s)).toBe(false);
	});

	it('borne le nombre de G-code retenus et tolère un stockage absent ou corrompu', () => {
		const s = memory();
		for (let i = 0; i < 40; i++) rememberSliceSignature('p', 'g' + i, 's' + i, s, i);
		expect(Object.keys(JSON.parse(s.data.get('web-slicer:slice-signatures:p')!))).toHaveLength(24);
		expect(restorableResult('p', 'g39', 's39', s)).toBe(true);
		expect(restorableResult('p', 'g0', 's0', s)).toBe(false);
		s.data.set('web-slicer:slice-signatures:p', '{pas du json');
		expect(restorableResult('p', 'g39', 's39', s)).toBe(false);
		expect(restorableResult('p', 'g', 's', null)).toBe(false);
		expect(() => rememberSliceSignature('p', 'g', 's', null)).not.toThrow();
	});
});

describe('serverSliceFreshness (API du lot 42, si disponible)', () => {
	const plates =
		(fresh: boolean, gcode = 'g1') =>
		async () => ({
			plates: [{ plate_index: 0, last_gcode_id: gcode, fresh }]
		});
	it('suit le verdict « fresh » du serveur pour le bon G-code', async () => {
		expect(await serverSliceFreshness('p', 0, 'g1', plates(true))).toBe(true);
		expect(await serverSliceFreshness('p', 0, 'g1', plates(false))).toBe(false);
		expect(await serverSliceFreshness('p', 0, 'g1', plates(true, 'g2'))).toBe(false);
		expect(await serverSliceFreshness('p', 1, 'g1', plates(true))).toBe(false);
	});
	it('backend sans la route (404/405) → null (repli sur l’empreinte locale)', async () => {
		const missing = async () => {
			throw new ApiError(404, 'not_found', 'not found');
		};
		expect(await serverSliceFreshness('p', 0, 'g1', missing)).toBeNull();
		const denied = async () => {
			throw new ApiError(500, 'internal', 'boom');
		};
		expect(await serverSliceFreshness('p', 0, 'g1', denied)).toBe(false);
	});
});

describe('décision de restauration (lot 42 V1, autre navigateur)', () => {
	it('autre navigateur (aucune empreinte) : le verdict serveur suffit', () => {
		expect(shouldRestore(true, { known: false, matches: false })).toBe(true);
		expect(shouldRestore(false, { known: false, matches: false })).toBe(false);
	});
	it('même navigateur : une empreinte différente (brouillon) l’emporte sur « à jour »', () => {
		expect(shouldRestore(true, { known: true, matches: false })).toBe(false);
		expect(shouldRestore(true, { known: true, matches: true })).toBe(true);
		expect(shouldRestore(false, { known: true, matches: true })).toBe(false);
	});
	it('serveur muet : empreinte locale seule', () => {
		expect(shouldRestore(null, { known: true, matches: true })).toBe(true);
		expect(shouldRestore(null, { known: false, matches: false })).toBe(false);
	});
	it('hasRememberedSignature', () => {
		const s = memory();
		expect(hasRememberedSignature('p', 'g1', s)).toBe(false);
		rememberSliceSignature('p', 'g1', 'sig', s);
		expect(hasRememberedSignature('p', 'g1', s)).toBe(true);
		expect(hasRememberedSignature('p', 'g2', s)).toBe(false);
		expect(hasRememberedSignature('p', 'g1', null)).toBe(false);
	});
});

describe('job en cours au rechargement (phase 42c)', () => {
	it('empreinte mémorisée au lancement sous l’id du job, retrouvée avant le G-code', () => {
		const s = memory();
		rememberSliceSignature('p', 'job-1', 'plateau-A', s);
		// G-code pas encore connu (ou jamais mémorisé) : repli sur l'id du job.
		expect(localSignatureVerdict('p', [undefined, 'job-1'], 'plateau-A', s)).toEqual({
			known: true,
			matches: true
		});
		expect(localSignatureVerdict('p', ['g9', 'job-1'], 'plateau-B', s)).toEqual({
			known: true,
			matches: false
		});
		// L'empreinte du G-code, si elle existe, prime sur celle du job.
		rememberSliceSignature('p', 'g9', 'plateau-B', s);
		expect(localSignatureVerdict('p', ['g9', 'job-1'], 'plateau-B', s).matches).toBe(true);
		expect(localSignatureVerdict('p', ['g0', 'job-0'], 'plateau-A', s)).toEqual({
			known: false,
			matches: false
		});
	});
	it('verdict serveur : signature du job = signature actuelle du plateau', async () => {
		const plates = (signature: string) => async () => ({
			plates: [{ plate_index: 0, fresh: false, signature }]
		});
		expect(await serverJobFreshness('p', 0, 'v1:a', plates('v1:a'))).toBe(true);
		// Projet modifié et enregistré depuis le lancement : job ignoré.
		expect(await serverJobFreshness('p', 0, 'v1:a', plates('v1:b'))).toBe(false);
		// Plateau vidé / supprimé.
		expect(await serverJobFreshness('p', 1, 'v1:a', plates('v1:a'))).toBe(false);
		// Job antérieur au lot 42 (pas de signature) ou route absente : null.
		expect(await serverJobFreshness('p', 0, undefined, plates('v1:a'))).toBeNull();
		const missing = async () => {
			throw new ApiError(405, 'method_not_allowed', 'nope');
		};
		expect(await serverJobFreshness('p', 0, 'v1:a', missing)).toBeNull();
		const boom = async () => {
			throw new ApiError(500, 'internal', 'boom');
		};
		expect(await serverJobFreshness('p', 0, 'v1:a', boom)).toBe(false);
	});
});
