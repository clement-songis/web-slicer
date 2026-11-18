import { describe, expect, it } from 'vitest';
import type { PrinterResponse } from '$lib/api/types';
import {
	HOST_TYPES,
	emptyHostForm,
	hasLiveControls,
	hostFormError,
	hostFormOf,
	hostMeta,
	hostRequest,
	normalizeHostUrl,
	testMessage
} from './hosts';

const printer = (p: Partial<PrinterResponse>): PrinterResponse => ({
	id: 'p',
	name: 'P',
	has_api_key: false,
	machine_preset_id: 'm',
	host_type: 'moonraker',
	has_host_password: false,
	...p
});

describe('hôtes d impression (PhysicalPrinterDialog)', () => {
	it('14 types du serveur, Moonraker par défaut', () => {
		expect(HOST_TYPES.map((h) => h.id).sort()).toEqual([
			'astrobox',
			'crealityprint',
			'duet',
			'elegoolink',
			'esp3d',
			'flashair',
			'flashforge',
			'mks',
			'moonraker',
			'obico',
			'octoprint',
			'prusaconnect',
			'prusalink',
			'repetier'
		]);
		expect(hostMeta(undefined).id).toBe('moonraker');
		expect(hostMeta('OctoPrint').id).toBe('octoprint');
		expect(hostMeta('simplyprint').id).toBe('moonraker');
	});

	it('« mettre en file » : PrusaConnect seulement (get_post_upload_actions)', () => {
		expect(HOST_TYPES.filter((h) => h.queue).map((h) => h.id)).toEqual(['prusaconnect']);
		expect(hostMeta('prusalink').queue).toBeFalsy();
	});

	it('suivi et contrôles : Moonraker connecté seulement', () => {
		expect(hasLiveControls(printer({ moonraker_url: 'http://k:7125' }))).toBe(true);
		expect(hasLiveControls(printer({}))).toBe(false);
		expect(hasLiveControls(printer({ host_type: 'octoprint', moonraker_url: 'http://o' }))).toBe(
			false
		);
	});

	it('adresse normalisée et validation', () => {
		expect(normalizeHostUrl(' 192.168.1.3/ ')).toBe('http://192.168.1.3');
		expect(normalizeHostUrl('https://connect.prusa3d.com')).toBe('https://connect.prusa3d.com');
		expect(hostFormError(emptyHostForm())).toBeNull();
		expect(hostFormError(emptyHostForm('octoprint'))).toMatch(/OctoPrint requise/);
		expect(hostFormError({ ...emptyHostForm('prusalink'), url: 'h', auth: 'user' })).toBe(
			'Utilisateur requis (absent du profil machine).'
		);
		expect(hostFormError({ ...emptyHostForm('octoprint'), url: 'http://a b' })).toBe(
			'Adresse invalide.'
		);
	});

	it('corps de requête : champs du type seulement, secrets vides conservés', () => {
		expect(
			hostRequest({
				...emptyHostForm('octoprint'),
				url: 'octopi.local',
				apiKey: 'K',
				user: 'x',
				slug: 'y'
			})
		).toEqual({
			host_type: 'octoprint',
			moonraker_url: 'http://octopi.local',
			api_key: 'K',
			host_auth: '',
			host_user: '',
			host_port: ''
		});
		expect(
			hostRequest({
				...emptyHostForm('prusalink'),
				url: 'h',
				auth: 'user',
				user: ' maker ',
				password: 'pw',
				apiKey: 'K'
			})
		).toEqual({
			host_type: 'prusalink',
			moonraker_url: 'http://h',
			host_auth: 'user',
			host_user: 'maker',
			host_password: 'pw',
			host_port: ''
		});
		const rep = hostRequest({ ...emptyHostForm('repetier'), url: 'r:3344', slug: 'ender3' });
		expect(rep.host_port).toBe('ender3');
		expect(rep).not.toHaveProperty('api_key');
		expect(
			hostRequest({ ...emptyHostForm('mks'), url: 'm', apiKey: 'ignored' })
		).not.toHaveProperty('api_key');
	});

	it('formulaire depuis une imprimante (secrets jamais pré-remplis)', () => {
		expect(
			hostFormOf(
				printer({
					host_type: 'repetier',
					moonraker_url: 'http://r',
					host_port: 'mk3',
					has_api_key: true
				})
			)
		).toEqual({
			type: 'repetier',
			url: 'http://r',
			apiKey: '',
			auth: 'key',
			user: '',
			password: '',
			slug: 'mk3',
			serial: ''
		});
		expect(
			hostFormOf(printer({ host_type: 'prusalink', host_auth: 'user', host_user: 'u' })).auth
		).toBe('user');
	});

	it('message de test selon le type', () => {
		expect(
			testMessage('moonraker', {
				connected: true,
				klippy_state: 'ready',
				moonraker_version: 'v0.9'
			})
		).toBe('Connecté (Klipper ready, Moonraker v0.9)');
		expect(
			testMessage('octoprint', {
				connected: true,
				klippy_state: 'OctoPrint',
				moonraker_version: '1.10.2'
			})
		).toBe('Connecté à OctoPrint 1.10.2');
	});
});
