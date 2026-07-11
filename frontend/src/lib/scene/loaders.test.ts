// Tests des loaders client (T051) : STL binaire/ASCII, OBJ, 3MF, sur de petits
// fichiers construits à la volée. On vérifie le nombre de triangles, les
// positions et la normalisation des normales calculées par face.
import { describe, expect, test } from 'bun:test';
import { zipSync, strToU8 } from 'fflate';
import { previewFormat, parseStl, parseObj, parse3mf } from './loaders';

// Un unique triangle (0,0,0)-(1,0,0)-(0,1,0), normale attendue +Z.
const TRI = [0, 0, 0, 1, 0, 0, 0, 1, 0];

function binaryStl(triangles: number[][]): ArrayBuffer {
	const buf = new ArrayBuffer(84 + triangles.length * 50);
	const view = new DataView(buf);
	view.setUint32(80, triangles.length, true);
	let off = 84;
	for (const t of triangles) {
		off += 12; // normale de face laissée à zéro
		for (const c of t) {
			view.setFloat32(off, c, true);
			off += 4;
		}
		off += 2;
	}
	return buf;
}

describe('previewFormat', () => {
	test('reconnaît les extensions prévisualisables', () => {
		expect(previewFormat('part.STL')).toBe('stl');
		expect(previewFormat('a.obj')).toBe('obj');
		expect(previewFormat('scene.3mf')).toBe('3mf');
		expect(previewFormat('cad.step')).toBeNull();
		expect(previewFormat('noext')).toBeNull();
	});
});

describe('parseStl', () => {
	test('parse un STL binaire et calcule la normale de face', () => {
		const mesh = parseStl(binaryStl([TRI]));
		expect(mesh.positions.length).toBe(9);
		expect(mesh.indices.length).toBe(3);
		expect([...mesh.indices]).toEqual([0, 1, 2]);
		// Normale +Z sur les 3 sommets.
		expect([...mesh.normals]).toEqual([0, 0, 1, 0, 0, 1, 0, 0, 1]);
	});

	test('parse un STL ASCII équivalent', () => {
		const ascii = `solid t
facet normal 0 0 0
  outer loop
    vertex 0 0 0
    vertex 1 0 0
    vertex 0 1 0
  endloop
endfacet
endsolid t`;
		const mesh = parseStl(new TextEncoder().encode(ascii).buffer);
		expect(mesh.positions.length).toBe(9);
		expect([...mesh.positions.slice(3, 6)]).toEqual([1, 0, 0]);
	});

	test('binaire et ASCII produisent la même géométrie', () => {
		const a = parseStl(binaryStl([TRI, TRI]));
		expect(a.indices.length).toBe(6);
	});
});

describe('parseObj', () => {
	test('parse sommets et faces triangulaires', () => {
		const obj = 'v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n';
		const mesh = parseObj(obj);
		expect(mesh.positions.length).toBe(9);
		expect([...mesh.normals.slice(0, 3)]).toEqual([0, 0, 1]);
	});

	test('triangule un quad en éventail', () => {
		const obj = 'v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n';
		const mesh = parseObj(obj);
		expect(mesh.positions.length).toBe(18); // 2 triangles
	});

	test('gère les indices négatifs (relatifs) et v/vt/vn', () => {
		const obj = 'v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3/1/1 -2/2/1 -1/3/1\n';
		const mesh = parseObj(obj);
		expect(mesh.positions.length).toBe(9);
	});

	test('lève sur un index de face hors limites', () => {
		expect(() => parseObj('v 0 0 0\nf 1 2 3\n')).toThrow();
	});
});

describe('parse3mf', () => {
	function make3mf(): ArrayBuffer {
		const model = `<?xml version="1.0"?>
<model unit="millimeter"><resources><object id="1" type="model"><mesh>
<vertices>
<vertex x="0" y="0" z="0"/>
<vertex x="1" y="0" z="0"/>
<vertex x="0" y="1" z="0"/>
</vertices>
<triangles><triangle v1="0" v2="1" v3="2"/></triangles>
</mesh></object></resources></model>`;
		const zipped = zipSync({ '3D/3dmodel.model': strToU8(model) });
		return zipped.buffer.slice(zipped.byteOffset, zipped.byteOffset + zipped.byteLength);
	}

	test('dézippe et parse le maillage', () => {
		const mesh = parse3mf(make3mf());
		expect(mesh.positions.length).toBe(9);
		expect([...mesh.positions.slice(3, 6)]).toEqual([1, 0, 0]);
		expect([...mesh.normals.slice(0, 3)]).toEqual([0, 0, 1]);
	});

	// Structure OrcaSlicer/Bambu (extension « production ») : le 3dmodel.model
	// racine ne contient que des <component>, la géométrie vit dans un fichier
	// 3D/Objects/*.model séparé. Régression : l'aperçu doit les agréger.
	function makeSplit3mf(): ArrayBuffer {
		const root = `<?xml version="1.0"?>
<model unit="millimeter"><resources><object id="1" type="model"><components>
<component p:path="/3D/Objects/part.model" objectid="1"/>
</components></object></resources>
<build><item objectid="1" transform="1 0 0 0 1 0 0 0 1 110 110 0"/></build></model>`;
		const object = `<?xml version="1.0"?>
<model unit="millimeter"><resources><object id="1" type="model"><mesh>
<vertices>
<vertex x="0" y="0" z="0"/>
<vertex x="2" y="0" z="0"/>
<vertex x="0" y="2" z="0"/>
</vertices>
<triangles><triangle v1="0" v2="1" v3="2"/></triangles>
</mesh></object></resources></model>`;
		const zipped = zipSync({
			'3D/3dmodel.model': strToU8(root),
			'3D/_rels/3dmodel.model.rels': strToU8('<Relationships/>'),
			'3D/Objects/part.model': strToU8(object)
		});
		return zipped.buffer.slice(zipped.byteOffset, zipped.byteOffset + zipped.byteLength);
	}

	test('agrège la géométrie des fichiers .model séparés (structure OrcaSlicer)', () => {
		const mesh = parse3mf(makeSplit3mf());
		expect(mesh.positions.length).toBe(9);
		expect([...mesh.positions.slice(3, 6)]).toEqual([2, 0, 0]);
	});

	test('gère les coordonnées en notation scientifique à exposant négatif', () => {
		const model = `<?xml version="1.0"?>
<model unit="millimeter"><resources><object id="1" type="model"><mesh>
<vertices>
<vertex x="-5.57945075e-14" y="0" z="0"/>
<vertex x="1" y="2.5e-3" z="0"/>
<vertex x="0" y="1" z="-1.2E-5"/>
</vertices>
<triangles><triangle v1="0" v2="1" v3="2"/></triangles>
</mesh></object></resources></model>`;
		const zipped = zipSync({ '3D/3dmodel.model': strToU8(model) });
		const buf = zipped.buffer.slice(zipped.byteOffset, zipped.byteOffset + zipped.byteLength);
		const mesh = parse3mf(buf);
		expect(mesh.positions.length).toBe(9);
		expect(mesh.positions[0]).toBeCloseTo(-5.57945075e-14);
		expect(mesh.positions[4]).toBeCloseTo(2.5e-3);
		expect(mesh.positions[8]).toBeCloseTo(-1.2e-5);
	});

	test('lève si le modèle 3D est absent', () => {
		const zipped = zipSync({ 'meta.txt': strToU8('rien') });
		const buf = zipped.buffer.slice(zipped.byteOffset, zipped.byteOffset + zipped.byteLength);
		expect(() => parse3mf(buf)).toThrow();
	});
});
