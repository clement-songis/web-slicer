import { describe, expect, it } from 'vitest';
import { DraftStore, MemoryDraftBackend, draftKey } from './draft';

function store() {
	return new DraftStore(new MemoryDraftBackend());
}

describe('draftKey', () => {
	it('mappe le projet ou « new »', () => {
		expect(draftKey('abc')).toBe('abc');
		expect(draftKey(null)).toBe('new');
	});
});

describe('DraftStore', () => {
	it('sauvegarde puis relit un brouillon (horodaté)', async () => {
		const s = store();
		await s.save({ projectId: 'p1', name: 'Benchy', scene: { a: 1 }, activePresets: {} });
		const draft = await s.load('p1');
		expect(draft?.name).toBe('Benchy');
		expect(draft?.scene).toEqual({ a: 1 });
		expect(typeof draft?.savedAt).toBe('string');
	});

	it('isole les brouillons par projet', async () => {
		const s = store();
		await s.save({ projectId: 'p1', name: 'A', scene: {}, activePresets: {} });
		await s.save({ projectId: null, name: 'Nouveau', scene: {}, activePresets: {} });
		expect((await s.load('p1'))?.name).toBe('A');
		expect((await s.load(null))?.name).toBe('Nouveau');
		expect(await s.load('inconnu')).toBeUndefined();
	});

	it('clear supprime le brouillon', async () => {
		const s = store();
		await s.save({ projectId: 'p1', name: 'A', scene: {}, activePresets: {} });
		await s.clear('p1');
		expect(await s.load('p1')).toBeUndefined();
	});

	it('propose la restauration seulement si le brouillon est plus récent', async () => {
		const s = store();
		const server = '2026-07-10T10:00:00Z';

		// Brouillon antérieur → rien à restaurer.
		await s.save({
			projectId: 'p1',
			name: 'vieux',
			scene: {},
			activePresets: {},
			savedAt: '2026-07-10T09:00:00Z'
		});
		expect(await s.pendingRestore('p1', server)).toBeNull();

		// Brouillon postérieur → restauration proposée.
		await s.save({
			projectId: 'p1',
			name: 'récent',
			scene: {},
			activePresets: {},
			savedAt: '2026-07-10T11:00:00Z'
		});
		expect((await s.pendingRestore('p1', server))?.name).toBe('récent');

		// Pas de version serveur (brouillon « nouveau ») → toujours proposé.
		await s.save({ projectId: null, name: 'draft', scene: {}, activePresets: {} });
		expect((await s.pendingRestore(null, null))?.name).toBe('draft');
	});

	it('ne restaure rien en l’absence de brouillon', async () => {
		const s = store();
		expect(await s.pendingRestore('p1', null)).toBeNull();
	});
});
