import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';

// Messages attendus en français (interface traduite).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import type { PrinterResponse } from '$lib/api/types';
import {
	HOST_GROUPS,
	HOST_TYPES,
	credentialSummary,
	hostCapabilities,
	testFailure,
	testMessage,
	testOutcome
} from './hosts';

const printer = (p: Partial<PrinterResponse>): PrinterResponse => ({
	id: 'p',
	name: 'P',
	has_api_key: false,
	machine_preset_id: 'm',
	host_type: 'moonraker',
	has_host_password: false,
	moonraker_url: 'http://h',
	...p
});

describe('hôtes d impression : interface (phase 21)', () => {
	it('les 14 hôtes (dont Obico, Creality Print, Flashforge, ElegooLink), chacun avec une aide, regroupés sans doublon', () => {
		expect(HOST_TYPES).toHaveLength(14);
		for (const h of HOST_TYPES) expect(h.help.length).toBeGreaterThan(10);
		const ids = HOST_GROUPS.flatMap((g) => g.hosts.map((h) => h.id));
		expect(ids.sort()).toEqual(HOST_TYPES.map((h) => h.id).sort());
		expect(HOST_GROUPS[0].hosts.map((h) => h.id)).toEqual(['moonraker']);
	});

	it('capacités : suivi Moonraker seul, stockage PrusaLink/Repetier, FlashAir sans lancement', () => {
		const ok = (t: string) => Object.fromEntries(hostCapabilities(t).map((c) => [c.id, c.ok]));
		expect(ok('moonraker')).toEqual({ upload: true, start: true, storage: false, live: true });
		expect(ok('octoprint')).toEqual({ upload: true, start: true, storage: false, live: false });
		expect(ok('prusalink').storage).toBe(true);
		expect(hostCapabilities('repetier').find((c) => c.id === 'storage')?.label).toBe(
			'Choix du groupe'
		);
		expect(ok('flashair').start).toBe(false);
	});

	it('libellé de test : nom non répété quand la version le porte', () => {
		const r = { connected: true, klippy_state: 'OctoPrint', moonraker_version: 'OctoPrint 1.10.2' };
		expect(testMessage('octoprint', r)).toBe('Connecté à OctoPrint 1.10.2');
		expect(testMessage('octoprint', { ...r, moonraker_version: '1.10.2' })).toBe(
			'Connecté à OctoPrint 1.10.2'
		);
		expect(
			testMessage('duet', { connected: true, klippy_state: 'Duet', moonraker_version: 'RRF' })
		).toBe('Connecté à Duet RRF');
		expect(
			testOutcome('moonraker', {
				connected: false,
				klippy_state: 'shutdown',
				moonraker_version: ''
			})
		).toEqual({
			ok: false,
			message: 'Injoignable (shutdown)'
		});
	});

	it('échec de test : cause du serveur, sinon message par hôte', () => {
		expect(testFailure('octoprint', 'hôte : HTTP 403 {"error":"Invalid API key"}')).toEqual({
			ok: false,
			message:
				'Échec — identifiants refusés par OctoPrint (HTTP 403). Vérifiez la clé ou le mot de passe.'
		});
		expect(testFailure('duet', 'hôte injoignable')).toEqual({
			ok: false,
			message: 'Échec — hôte injoignable'
		});
		expect(testFailure('prusalink', '')).toEqual({
			ok: false,
			message: 'Échec de connexion à PrusaLink'
		});
	});

	it('identifiants enregistrés résumés sans secret', () => {
		expect(
			credentialSummary(printer({ moonraker_url: undefined, host_type: 'octoprint' }))
		).toEqual([]);
		expect(credentialSummary(printer({ host_type: 'octoprint', has_api_key: true }))).toEqual([
			{ text: 'Clé API enregistrée' }
		]);
		expect(credentialSummary(printer({ host_type: 'octoprint' }))).toEqual([
			{ text: 'Clé API manquante', missing: true }
		]);
		expect(credentialSummary(printer({ host_type: 'moonraker' }))).toEqual([]); // clé facultative
		expect(
			credentialSummary(
				printer({
					host_type: 'prusalink',
					host_auth: 'user',
					host_user: 'maker',
					has_host_password: true
				})
			)
		).toEqual([
			{ text: 'Utilisateur « maker »', missing: false },
			{ text: 'Mot de passe enregistré' }
		]);
		expect(
			credentialSummary(printer({ host_type: 'repetier', has_api_key: true, host_port: 'mk3s' }))
		).toEqual([{ text: 'Clé API enregistrée' }, { text: 'Imprimante « mk3s »' }]);
		expect(credentialSummary(printer({ host_type: 'repetier', has_api_key: true }))[1]).toEqual({
			text: 'Imprimante du serveur à choisir',
			missing: true
		});
		expect(credentialSummary(printer({ host_type: 'esp3d' }))).toEqual([]);
	});
});
