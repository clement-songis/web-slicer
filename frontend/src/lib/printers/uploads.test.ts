import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { ApiError } from '$lib/api/client';
import type { PrinterUploadEntry } from '$lib/api/types';
import {
	formatUploadSize,
	hasPendingUploads,
	joinUploadName,
	uploadActionLabel,
	uploadNameErrorReason,
	uploadNameProblem,
	uploadNameReasonMessage,
	uploadNameStem,
	uploadPrinterIds,
	uploadStatusLabel
} from './uploads';
import {
	hostUrlError,
	hostUrlReasonMessage,
	isPrinterNotConnected,
	sendErrorMessage
} from './send';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

const entry = (over: Partial<PrinterUploadEntry>): PrinterUploadEntry =>
	({
		id: 'u1',
		printer_id: 'p1',
		filename: 'a.gcode',
		host: 'http://x',
		host_type: 'moonraker',
		size_bytes: 10n,
		action: 'upload',
		status: 'completed',
		print_started: false,
		created_at: '2026-11-21T10:00:00Z',
		...over
	}) as PrinterUploadEntry;

describe('nom de fichier d\u2019envoi (lot 42 M2)', () => {
	it('sépare la partie modifiable de l\u2019extension imposée', () => {
		expect(uploadNameStem('Cube_PLA_12m.gcode', '.gcode')).toBe('Cube_PLA_12m');
		expect(uploadNameStem('Cube.GCODE.3MF', '.gcode.3mf')).toBe('Cube');
		expect(uploadNameStem('sans-ext', '.gcode')).toBe('sans-ext');
	});
	it('rétablit l\u2019extension comme Orca, sans la doubler', () => {
		expect(joinUploadName(' Essai ', '.gcode')).toBe('Essai.gcode');
		expect(joinUploadName('Essai.gcode', '.gcode')).toBe('Essai.gcode');
		expect(joinUploadName('Été', '.gcode.3mf')).toBe('Été.gcode.3mf');
	});
	it('contrôle local : mêmes raisons que le serveur', () => {
		expect(uploadNameProblem('a/b', '.gcode')).toBe('chars');
		expect(uploadNameProblem('a:b')).toBe('chars');
		expect(uploadNameProblem('   ')).toBe('empty');
		expect(uploadNameProblem('.cache')).toBe('hidden');
		expect(uploadNameProblem('é'.repeat(100), '.gcode')).toBe('too_long');
		expect(uploadNameProblem('Pièce test', '.gcode')).toBeNull();
	});
	it('raisons traduites', () => {
		expect(uploadNameReasonMessage('chars')).toContain('caractères interdits');
		expect(uploadNameReasonMessage('empty')).toBe('Saisissez un nom de fichier.');
		expect(uploadNameReasonMessage('hidden')).toContain('point');
		expect(uploadNameReasonMessage('too_long')).toContain('200 octets');
	});
	it('lit la raison d\u2019un 422 serveur sur le nom', () => {
		const e = new ApiError(422, 'validation', 'x', { field: 'filename', reason: 'hidden' });
		expect(uploadNameErrorReason(e)).toBe('hidden');
		expect(
			uploadNameErrorReason(new ApiError(422, 'validation', 'x', { field: 'name' }))
		).toBeNull();
		expect(sendErrorMessage(e, 'Voron')).toContain('point');
	});
});

describe('adresse d\u2019hôte refusée par le serveur (lot 42 M1)', () => {
	it.each([
		['whitespace', 'espace'],
		['scheme', 'http://'],
		['host', 'adresse IP'],
		['port', '65535'],
		['query', '« ? »']
	])('raison %s traduite avec exemples', (reason, part) => {
		const e = new ApiError(422, 'validation', 'msg serveur', {
			field: 'moonraker_url',
			reason,
			value: 'pas une url'
		});
		const msg = hostUrlError(e)!;
		expect(msg).toContain(part);
		expect(msg).toContain('« pas une url »');
		expect(msg).toContain('Exemples');
	});
	it('409 printer_not_connected : adresse à ressaisir', () => {
		const e = new ApiError(409, 'printer_not_connected', 'x', { field: 'moonraker_url' });
		expect(isPrinterNotConnected(e)).toBe(true);
		expect(hostUrlError(e)).toContain('saisissez-la');
		expect(sendErrorMessage(e, 'Voron')).toContain('saisissez-la');
		const withReason = new ApiError(409, 'printer_not_connected', 'x', {
			field: 'moonraker_url',
			reason: 'whitespace',
			value: 'a b'
		});
		expect(hostUrlError(withReason)).toContain('espace');
	});
	it('autres erreurs : pas de message d\u2019adresse', () => {
		expect(hostUrlError(new ApiError(502, 'upstream', 'x'))).toBeNull();
		expect(hostUrlReasonMessage('autre')).toContain('adresse valable');
	});
});

describe('file d\u2019envoi (lot 42 M3)', () => {
	it('libellés d\u2019état et d\u2019action d\u2019Orca', () => {
		expect(uploadStatusLabel('uploading')).toBe('Téléversement');
		expect(uploadStatusLabel('completed')).toBe('Terminé');
		expect(uploadStatusLabel('error')).toBe('Erreur');
		expect(uploadActionLabel('print')).toBe('Téléverser et imprimer');
		expect(uploadActionLabel('queue')).toBe('Téléverser dans la file');
	});
	it('taille au format de la langue', () => {
		expect(formatUploadSize(3_456_789n)).toBe('3,3 Mo');
		expect(formatUploadSize(0)).toBe('—');
	});
	it('en cours et imprimantes présentes', () => {
		const list = [entry({}), entry({ id: 'u2', status: 'uploading', printer_id: 'p2' })];
		expect(hasPendingUploads(list)).toBe(true);
		expect(hasPendingUploads([entry({})])).toBe(false);
		expect(uploadPrinterIds([...list, entry({ id: 'u3' })])).toEqual(['p1', 'p2']);
	});
});
