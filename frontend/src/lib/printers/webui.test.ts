import { beforeEach, describe, expect, test } from 'vitest';
import {
	blockedMessage,
	embedBlockedReason,
	loadWebuiPrefs,
	presetWebui,
	saveWebuiPrefs,
	webuiUrl
} from './webui';

describe('interface web de l’hôte (PrintHost::get_print_host_webui d’Orca)', () => {
	test('print_host_webui, sinon print_host ; http:// ajouté sans schéma http/file', () => {
		expect(webuiUrl({ host: '192.168.1.20' })).toBe('http://192.168.1.20');
		expect(webuiUrl({ presetWebui: 'https://octo.local', host: '192.168.1.20' })).toBe(
			'https://octo.local'
		);
		expect(webuiUrl({ override: 'duet.local/ui', presetWebui: 'x' })).toBe('http://duet.local/ui');
		expect(webuiUrl({ host: 'HTTP://Prusa.local' })).toBe('HTTP://Prusa.local');
		expect(webuiUrl({ host: 'file:///tmp/a.html' })).toBe('file:///tmp/a.html');
		expect(webuiUrl({ host: '  ' })).toBe('');
		expect(webuiUrl({})).toBe('');
	});
	test('blocages connus : contenu mixte, fichier, adresse illisible', () => {
		expect(embedBlockedReason('http://octo.local', 'https:')).toBe('mixed');
		expect(embedBlockedReason('http://octo.local', 'http:')).toBeNull();
		expect(embedBlockedReason('https://octo.local', 'https:')).toBeNull();
		expect(embedBlockedReason('file:///a.html', 'http:')).toBe('file');
		expect(embedBlockedReason('http://', 'http:')).toBe('invalid');
		expect(blockedMessage('mixed')).toMatch(/HTTPS/);
	});
	test('print_host_webui du preset (chaîne ou tableau)', () => {
		expect(presetWebui({ print_host_webui: ' http://a ' })).toBe('http://a');
		expect(presetWebui({ print_host_webui: ['http://b'] })).toBe('http://b');
		expect(presetWebui({})).toBe('');
		expect(presetWebui(null)).toBe('');
	});
	describe('réglages locaux', () => {
		beforeEach(() => {
			const store = new Map<string, string>();
			globalThis.localStorage = {
				getItem: (k: string) => store.get(k) ?? null,
				setItem: (k: string, v: string) => void store.set(k, v),
				removeItem: (k: string) => void store.delete(k),
				clear: () => store.clear(),
				key: () => null,
				length: 0
			} as Storage;
		});
		test('aller-retour et nettoyage', () => {
			expect(loadWebuiPrefs('p')).toEqual({});
			saveWebuiPrefs('p', { url: ' http://x ', noEmbed: true });
			expect(loadWebuiPrefs('p')).toEqual({ url: 'http://x', noEmbed: true });
			saveWebuiPrefs('p', {});
			expect(loadWebuiPrefs('p')).toEqual({});
		});
	});
});
