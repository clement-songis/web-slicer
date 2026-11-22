// Phase 46 (audit 44 PR1-a/b) : lots de préréglages via les routes du lot 46.
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
	dispositionFilename,
	exportPresetBundleArchive,
	importPresetBundleFile
} from './preset-bundle';

afterEach(() => vi.unstubAllGlobals());

describe('dispositionFilename', () => {
	it('filename* UTF-8 prioritaire, puis filename, puis repli', () => {
		expect(
			dispositionFilename(
				'attachment; filename="Voron _.orca_printer"; filename*=UTF-8\'\'Voron%20%C3%A9.orca_printer',
				'x'
			)
		).toBe('Voron é.orca_printer');
		expect(dispositionFilename('attachment; filename="Presets.zip"', 'x')).toBe('Presets.zip');
		expect(dispositionFilename(null, 'x')).toBe('x');
	});
});

describe('exportPresetBundleArchive', () => {
	it('envoie ids + valeurs éditées, renvoie archive et nom', async () => {
		const fetchMock = vi.fn(
			async () =>
				new Response(new Uint8Array([80, 75]), {
					status: 200,
					headers: { 'content-disposition': 'attachment; filename="Voron Audit.orca_printer"' }
				})
		);
		vi.stubGlobal('fetch', fetchMock);
		const out = await exportPresetBundleArchive([
			{ id: 'a', values: { layer_height: 0.16 } },
			{ id: 'b' }
		]);
		expect(out?.filename).toBe('Voron Audit.orca_printer');
		const [url, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
		expect(url).toBe('/api/presets/export-bundle');
		expect(JSON.parse(String(init.body))).toEqual({
			presets: [{ id: 'a', values: { layer_height: 0.16 } }, { id: 'b' }]
		});
	});
	it('route absente → null ; refus → ApiError', async () => {
		vi.stubGlobal(
			'fetch',
			vi.fn(async () => new Response('', { status: 404 }))
		);
		expect(await exportPresetBundleArchive([{ id: 'a' }])).toBeNull();
		vi.stubGlobal(
			'fetch',
			vi.fn(
				async () =>
					new Response(
						JSON.stringify({ code: 'validation', message: 'aucun préréglage utilisateur' }),
						{
							status: 422
						}
					)
			)
		);
		await expect(exportPresetBundleArchive([{ id: 'a' }])).rejects.toMatchObject({ status: 422 });
	});
});

describe('importPresetBundleFile', () => {
	it('octets bruts + nom de fichier ; importés et ignorés', async () => {
		const fetchMock = vi.fn(
			async () =>
				new Response(
					JSON.stringify({
						imported: [{ id: '1', kind: 'process', name: 'P' }],
						skipped: [{ file: 'x.json', name: 'X', reason: 'préréglage parent « Y » introuvable' }]
					}),
					{ status: 200 }
				)
		);
		vi.stubGlobal('fetch', fetchMock);
		const out = await importPresetBundleFile(new File(['zip'], 'Voron é.orca_printer'));
		expect(out?.imported).toHaveLength(1);
		expect(out?.skipped[0].reason).toContain('introuvable');
		const [url] = fetchMock.mock.calls[0] as unknown as [string];
		expect(url).toBe('/api/presets/import-bundle?filename=Voron%20%C3%A9.orca_printer');
	});
});
