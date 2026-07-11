// Décodage des buffers de prévisualisation `WSPv` (T068, R6) produits par le
// backend (`GET /api/gcodes/{id}/preview/layers`). Le format est reconstruit à
// l'identique du côté Rust (`backend/src/gcode.rs`, `PreviewModel::encode_range`).
//
// En-tête (18 o, little-endian) :
//   magic "WSPv" | version u16 | from u32 | to u32 | count u32
// puis `count` enregistrements de 40 o :
//   start 3×f32 | end 3×f32 | feedrate f32 | width f32 | height f32
//   | kind u8 | extruder u8 | layer u16

/** Taille d'un enregistrement segment (octets), miroir de `PREVIEW_RECORD_BYTES`. */
export const RECORD_BYTES = 40;
/** Taille de l'en-tête (octets). */
export const HEADER_BYTES = 18;

const MAGIC = 'WSPv';

/**
 * Segments décodés sous forme de tableaux parallèles (structure de tableaux :
 * compact, prêt pour la construction de `BufferGeometry`).
 */
export interface PreviewSegments {
	/** Première et dernière couche incluses dans le buffer. */
	from: number;
	to: number;
	/** Nombre de segments. */
	count: number;
	/** Points de départ (x,y,z), longueur `count*3`. */
	start: Float32Array;
	/** Points d'arrivée (x,y,z), longueur `count*3`. */
	end: Float32Array;
	/** Vitesse commandée (mm/min), longueur `count`. */
	feedrate: Float32Array;
	/** Largeur de ligne (mm). */
	width: Float32Array;
	/** Hauteur de couche (mm). */
	height: Float32Array;
	/** Rôle d'extrusion (id, cf. `SEGMENT_ROLE_COLORS`). */
	kind: Uint8Array;
	/** Extrudeur / filament (0-based). */
	extruder: Uint8Array;
	/** Index de couche du segment. */
	layer: Uint16Array;
}

/** Erreur de format d'un buffer de prévisualisation. */
export class PreviewFormatError extends Error {}

/**
 * Décode un buffer binaire `WSPv` en tableaux parallèles. Lève
 * {@link PreviewFormatError} si le magic est absent ou la taille incohérente.
 */
export function decodePreview(buffer: ArrayBuffer): PreviewSegments {
	if (buffer.byteLength < HEADER_BYTES) {
		throw new PreviewFormatError('buffer de prévisualisation tronqué');
	}
	const view = new DataView(buffer);
	const magic = String.fromCharCode(
		view.getUint8(0),
		view.getUint8(1),
		view.getUint8(2),
		view.getUint8(3)
	);
	if (magic !== MAGIC) {
		throw new PreviewFormatError(`magic inattendu : ${magic}`);
	}
	// version = view.getUint16(4, true) — réservé (une seule version pour l'instant).
	const from = view.getUint32(6, true);
	const to = view.getUint32(10, true);
	const count = view.getUint32(14, true);

	const expected = HEADER_BYTES + count * RECORD_BYTES;
	if (buffer.byteLength < expected) {
		throw new PreviewFormatError(`buffer trop court : ${buffer.byteLength} < ${expected} attendus`);
	}

	const start = new Float32Array(count * 3);
	const end = new Float32Array(count * 3);
	const feedrate = new Float32Array(count);
	const width = new Float32Array(count);
	const height = new Float32Array(count);
	const kind = new Uint8Array(count);
	const extruder = new Uint8Array(count);
	const layer = new Uint16Array(count);

	let offset = HEADER_BYTES;
	for (let i = 0; i < count; i++) {
		start[i * 3] = view.getFloat32(offset, true);
		start[i * 3 + 1] = view.getFloat32(offset + 4, true);
		start[i * 3 + 2] = view.getFloat32(offset + 8, true);
		end[i * 3] = view.getFloat32(offset + 12, true);
		end[i * 3 + 1] = view.getFloat32(offset + 16, true);
		end[i * 3 + 2] = view.getFloat32(offset + 20, true);
		feedrate[i] = view.getFloat32(offset + 24, true);
		width[i] = view.getFloat32(offset + 28, true);
		height[i] = view.getFloat32(offset + 32, true);
		kind[i] = view.getUint8(offset + 36);
		extruder[i] = view.getUint8(offset + 37);
		layer[i] = view.getUint16(offset + 38, true);
		offset += RECORD_BYTES;
	}

	return { from, to, count, start, end, feedrate, width, height, kind, extruder, layer };
}
