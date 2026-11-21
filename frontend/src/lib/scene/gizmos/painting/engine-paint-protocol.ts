// Messages du WebSocket de peinture (GET /api/projects/{id}/paint), écrits à
// la main : ce ne sont pas des types ts-rs (cf. .work/staging/parity-audit/
// LOT38.md §3 et backend/src/http/routes/paint.rs). Tous les messages portent
// « seq » ; l'état « facets » (triangle → chaîne hex Orca) est toujours fourni
// par le client. Erreur : { type: 'error', seq, message } (message en français).
import type { PaintChannel } from './painting';

export type Vec3 = [number, number, number];

/** Forme du curseur (TriangleSelector::CursorType ; pointer = outil « Triangle »). */
export type EngineCursor = 'circle' | 'sphere' | 'pointer' | 'height_range';
/** Outil (ToolType d'Orca). */
export type EngineTool = 'brush' | 'smart_fill' | 'bucket_fill';

/** Trait (repère du maillage recentré de l'objet). */
export interface EngineStroke {
	points: Vec3[];
	facets: number[];
	camera_pos: Vec3;
	radius: number;
	cursor: EngineCursor;
	/** 0 efface, 1 enforcer, 2 blocker ; MMU : n° d'extrudeur (1-based). */
	state: number;
	/** Matrice monde colonne-major (16). */
	trafo: number[];
	/** Défaut serveur : vrai. */
	splitting?: boolean;
	/** « Surplombs seulement » (°), 0 = off. */
	highlight_deg?: number;
	tool?: EngineTool;
	/** Angle d'arrêt des remplissages (-1…90, défaut 30 ; -1 = pot sans détection d'arêtes). */
	fill_angle?: number;
	/** Plage de hauteur (mm, défaut 0,2). */
	height?: number;
	/**
	 * Plage de hauteur appliquée à une autre pièce de l'objet : cote monde du
	 * point touché ((trafo touché × hit).z), avec points: [] et facets: [].
	 */
	z_world?: number;
}

export interface StrokeRequest {
	type: 'stroke';
	object_id: string;
	channel: PaintChannel;
	facets: Record<string, string>;
	stroke: EngineStroke;
}

/** Comblement des interstices (bouton « Perform » d'Orca) ; area 0–5 mm². */
export interface GapFillRequest {
	type: 'gap_fill';
	object_id: string;
	channel: PaintChannel;
	facets: Record<string, string>;
	area: number;
}

/** Remappage MMU : map[i] = extrudeur destination (1–16) de l'extrudeur i + 1. */
export interface RemapRequest {
	type: 'remap';
	object_id: string;
	channel: 'mmu';
	facets: Record<string, string>;
	map: number[];
}

/** Aperçu au survol (smart_fill, bucket_fill, cursor pointer ; premier point). */
export interface PreviewRequest {
	type: 'preview';
	object_id: string;
	channel: PaintChannel;
	facets: Record<string, string>;
	stroke: EngineStroke;
}

/** Réponse des messages stroke / gap_fill / remap. */
export interface FacetsReply {
	type: 'facets';
	seq: number;
	object_id: string;
	channel: PaintChannel;
	facets: Record<string, string>;
}

/** Réponse de l'aperçu : 9 flottants par triangle sélectionné (repère maillage). */
export interface PreviewReply {
	type: 'preview';
	seq: number;
	object_id: string;
	channel: PaintChannel;
	vertices: number[];
	/** Vrai au-delà de 300 000 triangles. */
	truncated: boolean;
}

export interface ErrorReply {
	type: 'error';
	seq: number;
	message: string;
}
