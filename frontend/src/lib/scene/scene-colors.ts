// Palette de la vue 3D (design system v2). Les matériaux three.js ne lisent
// pas les variables CSS : ces tables sont la SEULE source des couleurs de
// scène (aucun hex codé en dur dans les .svelte). Une palette par thème, la
// vue suit la préférence de l'utilisateur (store `resolvedTheme`). Teintes
// alignées sur Orca (pièce grise, sélection ambre, volumes typés).
import { derived } from 'svelte/store';
import { resolvedTheme, type ResolvedTheme } from '$lib/theme/theme';

export interface ScenePalette {
	bed: string;
	bedOpacity: number;
	gridMajor: number;
	gridMinor: number;
	part: string;
	partSelected: string;
	partUnprintable: string;
	overhang: string;
	emissiveActive: string;
	emissiveNone: string;
	volumeNegative: string;
	volumeModifier: string;
	volumeBlocker: string;
	volumeEnforcer: string;
	sectionPlane: string;
	sectionPlaneHover: string;
	/** Points de mesure : 1er / 2e point (GLGizmoMeasure SELECTED_1ST/2ND_COLOR). */
	measureFirst: string;
	measureSecond: string;
	/** Objet désigné par la validation (phase 29) : lueur erreur / avertissement. */
	issueError: string;
	issueWarning: string;
}

const COMMON = {
	partSelected: '#f59e0b',
	overhang: '#ff4d4d',
	emissiveActive: '#f59e0b',
	emissiveNone: '#000000',
	volumeModifier: '#e8c547',
	volumeBlocker: '#e04848',
	volumeEnforcer: '#48c868',
	// Contrastent avec la teinte filament des objets (souvent ambre/orange).
	measureFirst: '#40bfbf',
	measureSecond: '#bf40bf',
	issueError: '#ff1f1f',
	issueWarning: '#ff8800'
} as const;

export const SCENE_PALETTES: Record<ResolvedTheme, ScenePalette> = {
	dark: {
		...COMMON,
		bed: '#1e293b',
		bedOpacity: 0.55,
		gridMajor: 0x64748b,
		gridMinor: 0x334155,
		part: '#9aa7b4',
		partUnprintable: '#555b63',
		volumeNegative: '#8c8c8c',
		sectionPlane: '#e5e7eb',
		sectionPlaneHover: '#38bdf8'
	},
	light: {
		...COMMON,
		// Plateau clair mais distinct du fond (surface-sunken #f1f5f9) ;
		// grille ≥ 3:1 sur le plateau pour rester lisible en atelier.
		bed: '#cbd5e1',
		bedOpacity: 0.7,
		gridMajor: 0x64748b,
		gridMinor: 0x94a3b8,
		part: '#7b8794',
		partUnprintable: '#b0b7bf',
		volumeNegative: '#6b7280',
		sectionPlane: '#475569',
		sectionPlaneHover: '#0284c7'
	}
};

/** Palette sombre (compatibilité : référence statique historique). */
export const SCENE_COLORS = SCENE_PALETTES.dark;

/** Palette de la vue 3D pour le thème effectivement appliqué. */
export const sceneColors = derived(resolvedTheme, (t) => SCENE_PALETTES[t]);
