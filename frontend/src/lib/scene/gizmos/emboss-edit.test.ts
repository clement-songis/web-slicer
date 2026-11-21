import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import { embossEditLabel, paramsFromEmboss } from './emboss';

beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));
import { deserializeObject, serializeObject } from '../document';
import { TrianglePainting } from './painting/painting';
import { BrimEars } from './brim-ears';
import { parseVolumes } from '../volumes';

describe('ré-édition du relief (objects[].emboss / volumes[].emboss)', () => {
	it('texte : paramètres de l’outil relus', () => {
		const p = paramsFromEmboss({
			type: 'text',
			text: 'Bonjour',
			font: 'noto-serif',
			font_file: 'x.ttf',
			style_name: 'r',
			size_mm: 12,
			depth_mm: 3,
			boldness_mm: 0.2
		});
		expect(p).toMatchObject({
			source: 'text',
			text: 'Bonjour',
			font: 'noto-serif',
			fontSize: 12,
			depth: 3,
			boldness: 0.2
		});
		expect(embossEditLabel({ type: 'text', text: 'a' })).toBe('Modifier le texte');
	});
	it('SVG : contenu, largeur, nom de fichier conservé', () => {
		const p = paramsFromEmboss({
			type: 'svg',
			svg: '<svg/>',
			file_name: 'logo.svg',
			width_mm: 40,
			depth_mm: 1.5
		});
		expect(p).toMatchObject({
			source: 'svg',
			svg: '<svg/>',
			svgWidth: 40,
			depth: 1.5,
			svgFileName: 'logo.svg'
		});
		expect(embossEditLabel({ type: 'svg', svg: '<svg/>' })).toBe('Modifier le SVG');
	});
	it('JSON inconnu : rien à ré-éditer', () => {
		expect(paramsFromEmboss(null)).toBeNull();
		expect(paramsFromEmboss({ type: 'text' })).toBeNull();
		expect(embossEditLabel({})).toBeNull();
	});
});

describe('emboss dans le document scène', () => {
	it('objet : sérialisé (même via un proxy), relu', () => {
		const emboss = new Proxy({ type: 'text', text: 'A' }, {});
		const doc = serializeObject({
			id: 'o',
			modelId: 'm',
			transform: { position: [0, 0, 0], rotation: [0, 0, 0], scale: [1, 1, 1] },
			extruder: 0,
			settings: {},
			painting: new TrianglePainting(),
			layerProfile: [],
			brimEars: new BrimEars(),
			printable: true,
			emboss
		});
		expect(doc.emboss).toEqual({ type: 'text', text: 'A' });
		expect(deserializeObject(doc).emboss).toEqual({ type: 'text', text: 'A' });
	});
	it('volume maillé : emboss conservé par parseVolumes', () => {
		const [v] = parseVolumes([
			{
				id: 'v',
				type: 'part',
				shape: 'mesh',
				modelId: 'm',
				size: [1, 1, 1],
				offset: [0, 0, 0],
				emboss: { type: 'svg', svg: '<svg/>' }
			}
		]);
		expect(v.emboss).toEqual({ type: 'svg', svg: '<svg/>' });
	});
});
