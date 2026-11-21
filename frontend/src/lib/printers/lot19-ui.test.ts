import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';

// Messages attendus en français (interface traduite).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import {
	HOST_TYPES,
	credentialSummary,
	emptyHostForm,
	hostCapabilities,
	hostFormError,
	hostMeta,
	hostFormOf,
	hostPrinterValue,
	hostRequest
} from './hosts';
import { elegooProtocol, profileHost, profilePlaceholder } from './hosts';
import { objectsBounds, printObjectLabel, printObjectState } from './device';

describe('hôtes lot 19 : Obico, Creality Print, Flashforge', () => {
	it('types présents (host_type d’Orca)', () => {
		const ids = HOST_TYPES.map((h) => h.id);
		expect(ids).toEqual(expect.arrayContaining(['obico', 'crealityprint', 'flashforge']));
		expect(hostMeta('obico').slugLabel).toBe("ID d'imprimante");
		expect(hostMeta('flashforge').serialNumber).toBe(true);
		expect(hostCapabilities('crealityprint').find((c) => c.id === 'start')?.ok).toBe(true);
	});
	it('Obico : ID extrait de « nom [id] » (printer_id = printhost_port)', () => {
		expect(hostPrinterValue('obico', 'Ender atelier [42]')).toBe('42');
		expect(hostPrinterValue('obico', 'sans-id')).toBe('sans-id');
		expect(hostPrinterValue('repetier', 'ender [3]')).toBe('ender [3]');
		const req = hostRequest({
			...emptyHostForm('obico'),
			url: 'app.obico.io',
			apiKey: 'TOK',
			slug: '42'
		});
		expect(req).toMatchObject({ host_type: 'obico', api_key: 'TOK', host_port: '42' });
	});
	it('Flashforge : numéro de série + code d’accès', () => {
		const f = { ...emptyHostForm('flashforge'), url: '192.168.1.70', apiKey: 'CODE' };
		expect(hostFormError(f)).toMatch(/Numéro de série requis/);
		expect(hostFormError({ ...f, apiKey: '' })).toBeNull();
		const req = hostRequest({ ...f, serial: ' SN123 ' });
		expect(req).toMatchObject({
			host_type: 'flashforge',
			api_key: 'CODE',
			host_serial_number: 'SN123',
			host_port: ''
		});
		expect(hostRequest({ ...f, apiKey: '' }).host_serial_number).toBeUndefined();
		// Numéro enregistré renvoyé par le serveur : formulaire pré-rempli.
		const saved = {
			id: 'p',
			name: 'AD5M',
			moonraker_url: 'http://192.168.1.70',
			has_api_key: true,
			machine_preset_id: 'm',
			host_type: 'flashforge',
			has_host_password: false,
			host_serial_number: 'SN123'
		};
		expect(hostFormOf(saved).serial).toBe('SN123');
		expect(hostRequest(hostFormOf(saved)).host_serial_number).toBe('SN123');
		const base = { moonraker_url: 'http://x', host_type: 'flashforge', has_host_password: false };
		expect(credentialSummary({ ...base, has_api_key: true })).toEqual([
			{ text: "Code d'accès enregistré" }
		]);
		expect(credentialSummary({ ...base, has_api_key: false })).toEqual([
			{ text: 'Sans code : console 8899' }
		]);
	});
});

describe('objets de l’impression (exclude_object)', () => {
	const s = { excluded_objects: ['CUBE.STL_ID_1_COPY_0'], current_object: 'CUBE.STL_ID_0_COPY_1' };
	it('libellé et état', () => {
		expect(printObjectLabel('cube.stl_id_0_copy_0')).toBe('cube.stl');
		expect(printObjectLabel('CUBE.STL_ID_0_COPY_1')).toBe('CUBE.STL · copie 2');
		expect(printObjectLabel('libre')).toBe('libre');
		expect(printObjectState('cube.stl_id_1_copy_0', s)).toBe('excluded');
		expect(printObjectState('CUBE.STL_ID_0_COPY_1', s)).toBe('current');
		expect(printObjectState('X', s)).toBe('pending');
	});
	it('cadre des contours', () => {
		const b = objectsBounds([
			{
				polygon: [
					[0, 0],
					[100, 0],
					[100, 50],
					[0, 50]
				]
			},
			{ polygon: [], center: [200, 100] }
		])!;
		expect(b.minX).toBeLessThan(0);
		expect(b.minX + b.width).toBeGreaterThan(200);
		expect(objectsBounds([])).toBeNull();
	});
});

describe('ElegooLink et réglages d’hôte du profil machine', () => {
	it('type elegoolink, protocole selon printer_model (classify_printer_model)', () => {
		expect(hostMeta('elegoolink').label).toBe('ElegooLink');
		expect(elegooProtocol('Elegoo Centauri Carbon')).toBe('cc');
		expect(elegooProtocol('Elegoo Centauri Carbon 2')).toBe('cc2');
		expect(elegooProtocol('Elegoo Neptune 4')).toBe('octoprint');
		expect(elegooProtocol(undefined)).toBe('octoprint');
		const start = (m?: string) =>
			hostCapabilities('elegoolink', m).find((c) => c.id === 'start')?.ok;
		expect(start('Elegoo Centauri Carbon 2')).toBe(false);
		expect(start('Elegoo Centauri Carbon')).toBe(true);
	});
	it('profil machine : valeurs lues, placeholders, validation assouplie', () => {
		const p = profileHost({
			printhost_port: 'mk3',
			printhost_authorization_type: 'user',
			printhost_user: ['maker'],
			printhost_password: 'x',
			flashforge_serial_number: 'SN9',
			printer_model: 'Elegoo Centauri Carbon'
		});
		expect(p).toEqual({
			port: 'mk3',
			auth: 'user',
			user: 'maker',
			hasPassword: true,
			serial: 'SN9',
			model: 'Elegoo Centauri Carbon'
		});
		expect(profileHost({ printhost_port: '' })).toEqual({});
		expect(profilePlaceholder('mk3')).toBe('depuis le profil machine : mk3');
		expect(profilePlaceholder(undefined, 'slug Repetier')).toBe('slug Repetier');
		const ff = { ...emptyHostForm('flashforge'), url: '10.0.0.2', apiKey: 'C' };
		expect(hostFormError(ff)).not.toBeNull();
		expect(hostFormError(ff, { serial: 'SN9' })).toBeNull();
		const pl = { ...emptyHostForm('prusalink'), url: '10.0.0.3', auth: 'user' as const };
		expect(hostFormError(pl)).toMatch(/Utilisateur requis/);
		expect(hostFormError(pl, { user: 'maker' })).toBeNull();
	});
});
