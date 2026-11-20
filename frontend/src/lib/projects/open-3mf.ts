// Ouverture d'un 3MF comme projet (lot 34) : décision du mode d'ouverture
// (Orca Plater::open_3mf_file → determine_load_type + ProjectDropDialog),
// choix mémorisé et passation du rapport d'import à la page projet via
// sessionStorage (l'import crée un nouveau projet puis redirige). Pur →
// testable ; les pages orchestrent les dialogues et l'appel API.
import type { ProjectLoadBehaviour } from '$lib/settings/preferences';
import type { GcodeWarning, ImportLoadMode, ImportReport } from '$lib/api/types';

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
 * Titre et corps de l'avertissement `validate_presets` (boîtes « Modified
 * G-code » / « Customized Preset » d'Orca, Plater.cpp) ; les items sont
 * listés à part par le dialogue.
 */
export function warningContent(warning: GcodeWarning): { title: string; body: string } {
	if (warning.kind === 'customized_presets') {
		return {
			title: 'Presets personnalisés',
			body: 'Ce 3MF contient les presets filament ou imprimante personnalisés suivants. Confirmez que le G-code de ces presets est sûr pour éviter d’endommager la machine.'
		};
	}
	return {
		title: 'G-code modifié',
		body: 'Ce 3MF contient le G-code modifié suivant dans des presets filament ou imprimante. Confirmez que tout ce G-code modifié est sûr pour éviter d’endommager la machine.'
	};
}
