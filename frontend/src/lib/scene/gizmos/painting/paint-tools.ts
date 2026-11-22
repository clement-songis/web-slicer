// Outils des gizmos de peinture (lot 38, parité GLGizmoFdmSupports / Seam /
// FuzzySkin / MmuSegmentation d'Orca 2.4.1). Un seul sélecteur d'outil par
// canal ; le paramètre de l'outil actif s'affiche dessous (rayon, hauteur,
// angle, aire). Pur → testable ; la page n'assemble que les messages.
import type { PaintChannel } from './painting';
import type { EngineCursor, EngineTool } from './engine-paint-protocol';
import type { MeshPick } from '../../mesh';

// Libellés et infobulles : clés i18n anglaises, traduites au rendu ($t).

/**
 * Outil de la barre : « brush » = cercle, « sphere », « pointer » = Triangle
 * (un triangle par point), « height » = Plage de hauteur, « fill » =
 * remplissage (smart fill des supports / fuzzy, pot du MMU), « gap » =
 * Comblement des interstices (bouton « Combler », sans clic sur l'objet).
 */
export type PaintTool = 'brush' | 'sphere' | 'pointer' | 'height' | 'fill' | 'gap';

/**
 * Outils proposés par canal, dans l'ordre d'Orca :
 * - supports : Circle, Sphere, Fill, Gap Fill (GLGizmoFdmSupports.cpp:267) ;
 * - couture : Circle, Sphere (GLGizmoSeam.cpp:179) — « Remplissage » gardé en
 *   plus (smart fill, proposé avant le lot 38 et utilisé par un test gelé ;
 *   extension sans perte de fonction) ;
 * - fuzzy skin : Circle, Sphere, Triangle, Fill (GLGizmoFuzzySkin.cpp:199) ;
 * - MMU : Circle, Sphere, Triangle, Height Range, Fill, Gap Fill
 *   (GLGizmoMmuSegmentation.cpp:486).
 */
export const CHANNEL_TOOLS: Record<PaintChannel, readonly PaintTool[]> = {
	supports: ['brush', 'sphere', 'fill', 'gap'],
	seam: ['brush', 'sphere', 'fill'],
	fuzzy: ['brush', 'sphere', 'pointer', 'fill'],
	mmu: ['brush', 'sphere', 'pointer', 'height', 'fill', 'gap']
};

export const PAINT_TOOL_LABELS: Record<PaintTool, string> = {
	brush: 'Brush',
	sphere: 'Sphere',
	pointer: 'Triangle',
	height: 'Height range',
	fill: 'Fill||paint',
	gap: 'Gap fill'
};

/** Description courte de l'outil (infobulle et ligne d'aide sous le sélecteur). */
export const PAINT_TOOL_HINTS: Record<PaintTool, string> = {
	brush: 'Disc projected from the camera',
	sphere: 'Ball around the hit point (goes through the object)',
	pointer: 'Paints the triangle under the cursor',
	height: 'Paints the height slice above the hit point',
	fill: 'Fills the area up to sharp edges',
	gap: 'Fills small isolated areas of another color'
};

/** Outils disponibles sur un canal. */
export function toolsFor(channel: PaintChannel): readonly PaintTool[] {
	return CHANNEL_TOOLS[channel];
}

/** Outil courant ramené au pinceau s'il n'existe pas sur le canal. */
export function coerceTool(channel: PaintChannel, tool: PaintTool): PaintTool {
	return CHANNEL_TOOLS[channel].includes(tool) ? tool : 'brush';
}

/** Outil qui peint au clic / au glisser (tous sauf le comblement). */
export function paintsOnClick(tool: PaintTool): boolean {
	return tool !== 'gap';
}

/**
 * Outils dont Orca surligne la sélection au survol (is_selected_by_seed_fill,
 * GLGizmoPainterBase.cpp:895-960) : remplissages et triangle. Le serveur
 * n'accepte « preview » que pour eux (LOT38 §3).
 */
export function hasHoverPreview(tool: PaintTool): boolean {
	return tool === 'fill' || tool === 'pointer';
}

/**
 * Contrainte « Vertical / Horizontal » de la souris : MMU (cercle, sphère,
 * triangle : les deux cases) et couture (« Vertical » seule) ;
 * GLGizmoPainterBase fige x ou y de l'écran pendant le glisser.
 */
export type PointerConstraint = 'vertical' | 'horizontal' | null;

export function constraintsFor(channel: PaintChannel, tool: PaintTool): PointerConstraint[] {
	if (channel === 'mmu' && (tool === 'brush' || tool === 'sphere' || tool === 'pointer'))
		return ['vertical', 'horizontal'];
	if (channel === 'seam' && (tool === 'brush' || tool === 'sphere')) return ['vertical'];
	return [];
}

/**
 * Position écran contrainte : « Vertical » garde le x du premier point du
 * trait, « Horizontal » son y (Orca : m_vertical_only →
 * _mouse_position.x() = m_last_mouse_click.x()).
 */
export function constrainPointer(
	first: readonly [number, number],
	current: readonly [number, number],
	constraint: PointerConstraint
): [number, number] {
	if (constraint === 'vertical') return [first[0], current[1]];
	if (constraint === 'horizontal') return [current[0], first[1]];
	return [current[0], current[1]];
}

/** Aides du pinceau transmises à la vue 3D : survol et contrainte de souris. */
export interface PaintAssist {
	/** « Verticale / Horizontale » : x ou y de l'écran figé pendant le trait. */
	constraint?: PointerConstraint;
	/** Survol de l'objet (aperçu des remplissages) ; null en sortie. */
	onhover?: (pick: MeshPick | null) => void;
}

/** Aperçu au survol : triangles (9 flottants chacun, repère du maillage). */
export interface HoverPreview {
	positions: Float32Array;
	color: string;
}

/** Bornes des paramètres (curseurs d'Orca). */
export const RADIUS_MIN = 0.2;
export const RADIUS_MAX = 20;
/** Plage de hauteur : Orca 0,1–8 mm, pas 0,2 au Ctrl+molette ; défaut serveur 0,2. */
export const HEIGHT_MIN = 0.1;
export const HEIGHT_MAX = 8;
export const HEIGHT_DEFAULT = 0.2;
/** Comblement : 0–5 mm² pas 0,2 (GapAreaMin/Max/Step) ; défaut Orca 0 (aucun effet). */
export const GAP_AREA_MIN = 0;
export const GAP_AREA_MAX = 5;
export const GAP_AREA_STEP = 0.2;
/** Angle d'arrêt des remplissages : 0–90° (défaut Orca 30). */
export const FILL_ANGLE_MIN = 0;
export const FILL_ANGLE_MAX = 90;
export const FILL_ANGLE_DEFAULT = 30;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const tidy = (v: number) => Math.round(v * 1000) / 1000;

export function clampHeight(v: number): number {
	return Number.isFinite(v) ? tidy(clamp(v, HEIGHT_MIN, HEIGHT_MAX)) : HEIGHT_DEFAULT;
}
export function clampGapArea(v: number): number {
	if (!Number.isFinite(v)) return 0;
	return tidy(clamp(Math.round(v / GAP_AREA_STEP) * GAP_AREA_STEP, GAP_AREA_MIN, GAP_AREA_MAX));
}
export function clampFillAngle(v: number): number {
	return Number.isFinite(v)
		? Math.round(clamp(v, FILL_ANGLE_MIN, FILL_ANGLE_MAX))
		: FILL_ANGLE_DEFAULT;
}
export function clampRadius(v: number): number {
	return Number.isFinite(v) ? tidy(clamp(v, RADIUS_MIN, RADIUS_MAX)) : 2;
}

/** Paramètres réglables de la barre. */
export interface PaintParams {
	radius: number;
	height: number;
	fillAngle: number;
	/** Case « Détection des arêtes » du pot MMU (décochée → fill_angle = -1). */
	edgeDetection: boolean;
	gapArea: number;
	/** Supports : angle de « Seulement sur les surplombs en évidence » (°),
	 *  0 = peinture partout (Orca `m_paint_on_overhangs_only` décochée). */
	overhangOnlyDeg?: number;
}

export const PAINT_PARAM_DEFAULTS: PaintParams = {
	radius: 2,
	height: HEIGHT_DEFAULT,
	fillAngle: FILL_ANGLE_DEFAULT,
	edgeDetection: true,
	gapArea: 0
};

/**
 * Ctrl + molette : paramètre de l'outil actif (Orca : taille du pinceau,
 * hauteur de plage, angle du remplissage, aire du comblement).
 */
export function wheelParams(tool: PaintTool, p: PaintParams, up: boolean): PaintParams {
	const s = up ? 1 : -1;
	switch (tool) {
		case 'height':
			return { ...p, height: clampHeight(p.height + 0.2 * s) };
		case 'fill':
			return { ...p, fillAngle: clampFillAngle(p.fillAngle + 5 * s) };
		case 'gap':
			return { ...p, gapArea: clampGapArea(p.gapArea + GAP_AREA_STEP * s) };
		case 'pointer':
			return p;
		default:
			return { ...p, radius: clampRadius(p.radius + 0.2 * s) };
	}
}

/** Champs cursor / tool / fill_angle / height du message « stroke ». */
export interface StrokeToolFields {
	cursor: EngineCursor;
	tool: EngineTool;
	fill_angle?: number;
	height?: number;
	/** `highlight_by_angle_deg` de `TriangleSelector::select_patch` /
	 *  `seed_fill_select_triangles` (0 = sans restriction). */
	highlight_deg?: number;
}

/** Angle de surplomb (°) d'Orca : curseur 0–90 du gizmo Peinture de supports. */
export const OVERHANG_ANGLE_MIN = 0;
export const OVERHANG_ANGLE_MAX = 90;
export function clampOverhangAngle(deg: number): number {
	if (!Number.isFinite(deg)) return 0;
	return Math.min(OVERHANG_ANGLE_MAX, Math.max(OVERHANG_ANGLE_MIN, deg));
}

/**
 * Angle initial de « Mettre en évidence les zones en surplomb » à l'ouverture
 * du gizmo (Orca `get_selection_support_threshold_angle`,
 * GLGizmoFdmSupports.cpp:512-526) : seuil des supports automatiques s'ils sont
 * activés (`enable_support` et `support_type` normal/tree auto), sinon 0.
 */
export function supportHighlightAngle(values: Record<string, unknown>): number {
	const flag = (v: unknown) => v === true || v === 1 || v === '1' || v === 'true';
	const type = String(values.support_type ?? '');
	if (!flag(values.enable_support) || !/\(auto\)$/.test(type)) return 0;
	const angle = Number(values.support_threshold_angle);
	return Number.isFinite(angle) ? clampOverhangAngle(angle) : 0;
}

/**
 * Correspondance des outils d'Orca (LOT38 §3) : « Fill » des supports / fuzzy
 * / couture = smart_fill ; « Fill » du MMU = bucket_fill (+ « Edge
 * detection » → angle ou -1) ; « Triangle » = cursor pointer ; « Height
 * range » = cursor height_range.
 */
export function strokeFields(
	channel: PaintChannel,
	tool: PaintTool,
	p: PaintParams,
	hasCamera = true
): StrokeToolFields {
	const fields = toolFields(channel, tool, p, hasCamera);
	// Audit 46 SUPP-a : « On highlighted overhangs only » (GLGizmoPainterBase :
	// `m_paint_on_overhangs_only ? m_highlight_by_angle_threshold_deg : 0`).
	const deg = channel === 'supports' ? clampOverhangAngle(p.overhangOnlyDeg ?? 0) : 0;
	return deg > 0 ? { ...fields, highlight_deg: deg } : fields;
}

function toolFields(
	channel: PaintChannel,
	tool: PaintTool,
	p: PaintParams,
	hasCamera: boolean
): StrokeToolFields {
	switch (tool) {
		case 'fill':
			return channel === 'mmu'
				? {
						cursor: 'pointer',
						tool: 'bucket_fill',
						fill_angle: p.edgeDetection ? clampFillAngle(p.fillAngle) : -1
					}
				: { cursor: 'pointer', tool: 'smart_fill', fill_angle: clampFillAngle(p.fillAngle) };
		case 'pointer':
			return { cursor: 'pointer', tool: 'brush' };
		case 'height':
			return { cursor: 'height_range', tool: 'brush', height: clampHeight(p.height) };
		case 'sphere':
			return { cursor: 'sphere', tool: 'brush' };
		default:
			return { cursor: hasCamera ? 'circle' : 'sphere', tool: 'brush' };
	}
}

// — Remappage des filaments MMU (« Remap filaments ») ——————————————————

/**
 * Table d'Orca (m_extruder_remap, 0-based : table[src] = dst) → « map » du
 * message remap (1-based, map[i] = destination de l'extrudeur i + 1).
 * Identité → null (Orca : any_change faux, rien n'est fait).
 */
export function remapMessageMap(table: readonly number[]): number[] | null {
	const map = table.slice(0, 16).map((dst) => dst + 1);
	return map.some((dst, i) => dst !== i + 1) ? map : null;
}

/**
 * Extrudeur de base d'un objet / volume après remappage (Orca
 * remap_filament_assignments : extrudeur 0 = hérité → traité comme le 1) ;
 * null si inchangé.
 */
export function remapBaseExtruder(extruder: number, table: readonly number[]): number | null {
	const idx = extruder > 0 ? extruder - 1 : 0;
	if (idx >= table.length) return null;
	const dst = table[idx];
	return dst !== idx ? dst + 1 : null;
}

/**
 * Peinture MMU après fusion de filaments : « map » du message remap pour une
 * fonction de renumérotation 1-based (Orca TriangleSelector::deserialize avec
 * to_delete_filament / replace_filament) ; null si rien ne change.
 */
export function mergeRemapMap(
	slotCount: number,
	remap: (extruder: number) => number
): number[] | null {
	const n = Math.min(16, Math.max(1, slotCount));
	const map = Array.from({ length: n }, (_, i) => Math.max(1, remap(i + 1)));
	return map.some((dst, i) => dst !== i + 1) ? map : null;
}
