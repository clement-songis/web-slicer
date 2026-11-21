import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';

// Messages attendus en français (interface traduite).
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import { ApiError } from '$lib/api/client';
import type { PrinterResponse } from '$lib/api/types';
import {
	normalizeMoonrakerUrl,
	printerOptionLabel,
	sendChoices,
	sendErrorMessage,
	sendSuccessMessage,
	sendTargets,
	connectionHint,
	isBambuPrinter
} from './send';

const printer = (id: string, url: string | null) =>
	({ id, name: id, moonraker_url: url }) as unknown as PrinterResponse;

describe('sendTargets', () => {
	it('ne garde que les imprimantes avec URL Moonraker', () => {
		const out = sendTargets([printer('a', 'http://x:7125'), printer('b', null), printer('c', ' ')]);
		expect(out.map((p) => p.id)).toEqual(['a']);
	});
});

describe('sendChoices', () => {
	it('liste aussi les imprimantes non connectées, les connectées en premier', () => {
		const out = sendChoices([
			printer('setup', null),
			printer('a', 'http://x:7125'),
			printer('b', ' ')
		]);
		expect(out.map((p) => p.id)).toEqual(['a', 'setup', 'b']);
	});
	it('libellé : URL ou « non connectée »', () => {
		expect(printerOptionLabel(printer('Voron', 'http://x:7125'))).toBe('Voron — http://x:7125');
		expect(printerOptionLabel(printer('Voron', null))).toBe('Voron — non connectée');
	});
});

describe('normalizeMoonrakerUrl', () => {
	it('ajoute http:// et retire la barre finale', () => {
		expect(normalizeMoonrakerUrl(' 192.168.1.20:7125/ ')).toBe('http://192.168.1.20:7125');
		expect(normalizeMoonrakerUrl('https://voron.local')).toBe('https://voron.local');
	});
	it('refuse le vide et les schémas non http', () => {
		expect(normalizeMoonrakerUrl('  ')).toBeNull();
		expect(normalizeMoonrakerUrl('ftp://x')).toBeNull();
		expect(normalizeMoonrakerUrl('http://')).toBeNull();
	});
	it('audit 41 M1 : refuse un hôte avec espaces ou caractères invalides', () => {
		expect(normalizeMoonrakerUrl('pas une url')).toBeNull();
		expect(normalizeMoonrakerUrl('http://pas une url')).toBeNull();
		expect(normalizeMoonrakerUrl('http://voron!/')).toBeNull();
		expect(normalizeMoonrakerUrl('192.168.1')).toBeNull();
		expect(normalizeMoonrakerUrl('192.168.1.300:7125')).toBeNull();
		expect(normalizeMoonrakerUrl('voron.local:99999')).toBeNull();
		expect(normalizeMoonrakerUrl('http://a..b')).toBeNull();
	});
	it('audit 41 M1 : accepte noms, IPv4, IPv6, port, utilisateur et chemin', () => {
		expect(normalizeMoonrakerUrl('voron')).toBe('http://voron');
		expect(normalizeMoonrakerUrl('my_printer.lan:7125')).toBe('http://my_printer.lan:7125');
		expect(normalizeMoonrakerUrl('http://[fe80::1]:7125')).toBe('http://[fe80::1]:7125');
		expect(normalizeMoonrakerUrl('https://u:p@host.example/octo/')).toBe(
			'https://u:p@host.example/octo'
		);
		expect(normalizeMoonrakerUrl('127.0.0.1:17125')).toBe('http://127.0.0.1:17125');
	});
});

describe('sendSuccessMessage', () => {
	it('mentionne le démarrage seulement si honoré', () => {
		expect(sendSuccessMessage('Voron', { path: 'a.gcode', print_started: true })).toContain(
			'impression démarrée'
		);
		expect(sendSuccessMessage('Voron', { path: 'a.gcode', print_started: false })).not.toContain(
			'démarrée'
		);
	});
	it('mise en file (PrusaConnect to_queue)', () => {
		expect(sendSuccessMessage('MK4', { path: '/local/a.gcode', print_started: false }, true)).toBe(
			"« /local/a.gcode » envoyé à MK4 et mis en file d'attente."
		);
	});
});

describe('sendErrorMessage', () => {
	it('imprimante injoignable (erreur réseau relayée en 502)', () => {
		const e = new ApiError(
			502,
			'bad_gateway',
			'erreur réseau Moonraker : error sending request: connection refused'
		);
		expect(sendErrorMessage(e, 'Voron')).toMatch(/Voron est injoignable/);
	});
	it('refus HTTP : clé API pour 401/403, statut sinon', () => {
		expect(
			sendErrorMessage(new ApiError(502, 'bad_gateway', 'Moonraker a renvoyé le statut HTTP 401'))
		).toMatch(/clé API/);
		expect(
			sendErrorMessage(new ApiError(502, 'bad_gateway', 'Moonraker a renvoyé le statut HTTP 500'))
		).toMatch(/HTTP 500/);
	});
	it('réponse illisible, 404 et coupure côté navigateur', () => {
		expect(
			sendErrorMessage(new ApiError(502, 'bad_gateway', 'réponse Moonraker illisible : x'))
		).toMatch(/inattendue/);
		expect(sendErrorMessage(new ApiError(404, 'not_found', 'G-code'))).toMatch(
			/tranchez à nouveau/
		);
		expect(sendErrorMessage(new TypeError('Failed to fetch'))).toMatch(/web-slicer injoignable/);
	});
});

describe('connectionHint', () => {
	it('signale le réseau Bambu non pris en charge (nom ou preset)', () => {
		expect(isBambuPrinter({ name: 'Bambu Lab P1S' })).toBe(true);
		expect(isBambuPrinter({ name: 'Atelier', machine_preset_id: 'Bambu Lab H2D 0.4 nozzle' })).toBe(
			true
		);
		expect(connectionHint({ name: 'Bambu Lab P1S' })).toMatch(/réseau Bambu.*exportez le G-code/);
	});
	it('rien pour une imprimante Klipper ou Prusa', () => {
		expect(isBambuPrinter({ name: 'Voron 2.4 350' })).toBe(false);
		expect(connectionHint({ name: 'Prusa MK4' })).toBeNull();
	});
});
