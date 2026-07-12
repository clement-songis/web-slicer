import { describe, it, expect } from 'vitest';
import { sortByUpdated, formatDate, thumbnailUrl } from './library';
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
