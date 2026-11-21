import { afterAll, beforeAll, describe, it, expect } from 'vitest';
import { setLocale } from '$lib/i18n';
import {
	sortByUpdated,
	formatDate,
	thumbnailUrl,
	filterProjects,
	relativeTime,
	sortProjects
} from './library';
import type { ProjectResponse } from '$lib/api/types';

function project(over: Partial<ProjectResponse>): ProjectResponse {
	return {
		id: 'p1',
		name: 'Projet',
		version: 1n,
		scene: null,
		active_presets: null,
		has_thumbnail: false,
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z',
		...over
	};
}

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('sortByUpdated', () => {
	it('classe du plus récent au plus ancien sans muter la source', () => {
		const a = project({ id: 'a', updated_at: '2026-01-01T00:00:00Z' });
		const b = project({ id: 'b', updated_at: '2026-03-01T00:00:00Z' });
		const c = project({ id: 'c', updated_at: '2026-02-01T00:00:00Z' });
		const source = [a, b, c];
		expect(sortByUpdated(source).map((p) => p.id)).toEqual(['b', 'c', 'a']);
		expect(source.map((p) => p.id)).toEqual(['a', 'b', 'c']);
	});
});

describe('formatDate', () => {
	it('retombe sur la chaîne brute quand la date est invalide', () => {
		expect(formatDate('pas-une-date')).toBe('pas-une-date');
	});
});

describe('thumbnailUrl', () => {
	it("renvoie l'URL quand une vignette existe, sinon null", () => {
		expect(thumbnailUrl({ id: 'x', has_thumbnail: true })).toBe('/api/projects/x/thumbnail');
		expect(thumbnailUrl({ id: 'x', has_thumbnail: false })).toBeNull();
	});
});

describe('recherche, tri et dates relatives', () => {
	const p = (name: string, created: string, updated: string) =>
		({
			id: name,
			name,
			created_at: created,
			updated_at: updated,
			has_thumbnail: false
		}) as unknown as Parameters<typeof sortProjects>[0][number];
	const list = [
		p('Écrou', '2026-01-02T00:00:00Z', '2026-01-05T00:00:00Z'),
		p('cube', '2026-01-03T00:00:00Z', '2026-01-04T00:00:00Z'),
		p('Bride 10', '2026-01-01T00:00:00Z', '2026-01-06T00:00:00Z'),
		p('Bride 9', '2026-01-04T00:00:00Z', '2026-01-01T00:00:00Z')
	];
	it('tri par modification, nom (accents, nombres) et création', () => {
		expect(sortProjects(list, 'updated').map((x) => x.name)).toEqual([
			'Bride 10',
			'Écrou',
			'cube',
			'Bride 9'
		]);
		expect(sortProjects(list, 'name').map((x) => x.name)).toEqual([
			'Bride 9',
			'Bride 10',
			'cube',
			'Écrou'
		]);
		expect(sortProjects(list, 'created').map((x) => x.name)).toEqual([
			'Bride 9',
			'cube',
			'Écrou',
			'Bride 10'
		]);
	});
	it('recherche insensible à la casse et aux accents', () => {
		expect(filterProjects(list, 'ecr').map((x) => x.name)).toEqual(['Écrou']);
		expect(filterProjects(list, '  BRIDE ').length).toBe(2);
		expect(filterProjects(list, '').length).toBe(4);
	});
	it('dates relatives', () => {
		const now = Date.parse('2026-01-10T12:00:00Z');
		expect(relativeTime('2026-01-10T11:59:30Z', now)).toBe("à l'instant");
		expect(relativeTime('2026-01-10T11:55:00Z', now)).toBe('il y a 5 min');
		expect(relativeTime('2026-01-10T09:00:00Z', now)).toBe('il y a 3 h');
		expect(relativeTime('2026-01-09T12:00:00Z', now)).toBe('hier');
		expect(relativeTime('2026-01-06T12:00:00Z', now)).toBe('il y a 4 j');
		expect(relativeTime('pas une date', now)).toBe('pas une date');
	});
});
