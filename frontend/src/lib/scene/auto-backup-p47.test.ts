// Audit 45 PREF-b — sauvegarde automatique (Orca « Auto backup »).
import { beforeEach, describe, expect, test } from 'vitest';
import {
	BACKUP_PREFIX,
	clampBackupInterval,
	clearBackup,
	readBackup,
	shouldOfferRestore,
	writeBackup
} from './auto-backup';
import { DEFAULT_PREFERENCES, parsePreferences } from '$lib/settings/preferences';

const store = new Map<string, string>();
beforeEach(() => {
	store.clear();
	(globalThis as { localStorage?: unknown }).localStorage = {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v),
		removeItem: (k: string) => void store.delete(k)
	};
});

describe('auto-backup', () => {
	test('écriture / lecture / effacement', () => {
		writeBackup('p1', { version: '3', savedAt: 1, scene: { objects: [] }, activePresets: null });
		expect(store.has(BACKUP_PREFIX + 'p1')).toBe(true);
		expect(readBackup('p1')?.version).toBe('3');
		clearBackup('p1');
		expect(readBackup('p1')).toBeNull();
	});
	test('restauration proposée seulement sur la même version et un contenu différent', () => {
		const b = { version: '3', savedAt: 1, scene: { a: 1 }, activePresets: { p: 1 } };
		expect(shouldOfferRestore(b, '3', { a: 2 }, { p: 1 })).toBe(true);
		expect(shouldOfferRestore(b, '3', { a: 1 }, { p: 2 })).toBe(true);
		expect(shouldOfferRestore(b, '3', { a: 1 }, { p: 1 })).toBe(false);
		expect(shouldOfferRestore(b, '4', { a: 2 }, { p: 1 })).toBe(false);
		expect(shouldOfferRestore(null, '3', {}, {})).toBe(false);
	});
	test('intervalle borné, préférences d’Orca par défaut (activé, 10 s)', () => {
		expect(clampBackupInterval(0)).toBe(1);
		expect(clampBackupInterval('abc')).toBe(10);
		expect(DEFAULT_PREFERENCES.autoBackup).toBe(true);
		expect(DEFAULT_PREFERENCES.backupInterval).toBe(10);
		const p = parsePreferences(JSON.stringify({ autoBackup: false, backupInterval: 99999 }));
		expect(p.autoBackup).toBe(false);
		expect(p.backupInterval).toBe(3600);
	});
});
