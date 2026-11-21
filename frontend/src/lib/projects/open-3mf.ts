// Ouverture d'un 3MF comme projet (lot 34) : décision du mode d'ouverture
// (Orca Plater::open_3mf_file → determine_load_type + ProjectDropDialog),
// choix mémorisé et passation du rapport d'import à la page projet via
// sessionStorage (l'import crée un nouveau projet puis redirige). Pur →
// testable ; les pages orchestrent les dialogues et l'appel API.
import type { ProjectLoadBehaviour } from '$lib/settings/preferences';
import type { GcodeWarning, ImportLoadMode, ImportReport } from '$lib/api/types';
import { tr } from '$lib/i18n';

/** Fichier projet 3MF (les autres formats s'importent comme modèles). */
export function isProjectArchive(name: string): boolean {
	return /\.3mf$/i.test(name);
}

/**
 * Mode d'ouverture demandé par la préférence (determine_load_type d'Orca) :
 * `load_all` → projet, `load_geometry_only` → géométrie, `always_ask` →
 * dialogue, `ask_when_relevant` → dialogue seulement s'il y a un travail en
 * cours (Orca : plateau non vide ; ici : projet courant non vide ou
 * bibliothèque = rien à perdre).
 */
export function decideLoadMode(
	behaviour: ProjectLoadBehaviour,
	hasCurrentWork: boolean
): ImportLoadMode | 'ask' {
	switch (behaviour) {
		case 'load_geometry_only':
			return 'geometry';
		case 'always_ask':
			return 'ask';
		case 'ask_when_relevant':
			return hasCurrentWork ? 'ask' : 'project';
		default:
			return 'project';
	}
}

/**
 * Valeur de préférence à mémoriser pour un choix du dialogue (« Se souvenir
 * de mon choix », case masquée chez Orca 2.4.1) : le choix devient le
 * comportement permanent (`load_all` = toujours projet).
 */
export function behaviourFromChoice(mode: ImportLoadMode): ProjectLoadBehaviour {
	return mode === 'geometry' ? 'load_geometry_only' : 'load_all';
}

// — Passation du rapport d'import à la page projet —————————————————————
const REPORT_KEY = 'web-slicer.import-report';

/** Mémorise le rapport d'import du projet fraîchement créé (redirection). */
export function stashImportReport(projectId: string, report: ImportReport): void {
	try {
		sessionStorage.setItem(`${REPORT_KEY}:${projectId}`, JSON.stringify(report));
	} catch {
		// Stockage indisponible : l'avertissement n'est simplement pas rejoué.
	}
}

/** Lit (et retire) le rapport d'import en attente du projet affiché. */
export function takeImportReport(projectId: string): ImportReport | null {
	try {
		const raw = sessionStorage.getItem(`${REPORT_KEY}:${projectId}`);
		if (raw === null) return null;
		sessionStorage.removeItem(`${REPORT_KEY}:${projectId}`);
		const value = JSON.parse(raw) as Partial<ImportReport>;
		if (value.load !== 'project' && value.load !== 'geometry') return null;
		return {
			load: value.load,
			...(value.gcode_warning ? { gcode_warning: value.gcode_warning } : {}),
			has_metadata: value.has_metadata === true
		};
	} catch {
		return null;
	}
}

/**
 * Libellé (anglais d'Orca, traduit par le catalogue des réglages) d'une clé
 * G-code listée par l'avertissement « G-code modifié ». Orca n'affiche que la
 * clé brute (`-machine_start_gcode`) ; ici le nom de la page de réglages
 * accompagne la clé. `null` = clé inconnue (affichée seule).
 */
const GCODE_ITEM_LABELS: Record<string, string> = {
	machine_start_gcode: 'Machine start G-code',
	machine_end_gcode: 'Machine end G-code',
	filament_start_gcode: 'Filament start G-code',
	filament_end_gcode: 'Filament end G-code',
	change_filament_gcode: 'Change filament G-code',
	layer_change_gcode: 'Layer change G-code',
	before_layer_change_gcode: 'Before layer change G-code',
	machine_pause_gcode: 'Pause G-code',
	template_custom_gcode: 'Template Custom G-code',
	printing_by_object_gcode: 'Printing by object G-code',
	time_lapse_gcode: 'Timelapse G-code',
	wrapping_detection_gcode: 'Clumping Detection G-code'
};
export function gcodeItemLabel(key: string): string | null {
	return GCODE_ITEM_LABELS[key] ?? null;
}

/**
 * Titre et corps de l'avertissement `validate_presets` (boîtes « Modified
 * G-code » / « Customized Preset » d'Orca, Plater.cpp) ; les items sont
 * listés à part par le dialogue.
 */
export function warningContent(warning: GcodeWarning): { title: string; body: string } {
	if (warning.kind === 'customized_presets') {
		return {
			title: tr('Customized presets'),
			body: tr(
				'This 3MF contains the following customized filament or printer presets. Confirm that the G-code of these presets is safe to avoid damaging the machine.'
			)
		};
	}
	return {
		title: tr('Modified G-code'),
		body: tr(
			'This 3MF contains the following modified G-code in filament or printer presets. Confirm that all this modified G-code is safe to avoid damaging the machine.'
		)
	};
}
