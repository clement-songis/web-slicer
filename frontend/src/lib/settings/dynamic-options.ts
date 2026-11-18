// Lignes « dynamiques » du layout généré (`{ dynamic: 'option' }`) : options
// qu'OrcaSlicer ajoute dans Tab.cpp par `optgroup->get_option(<clé>)` puis
// `append_single_option_line(option)` (pleine largeur), forme que l'audit ne
// résout pas statiquement. Sans cette table, ces réglages (notes, vignettes)
// n'étaient pas éditables. Seules les lignes dont la clé est certaine sont
// résolues ; les autres marqueurs restent dynamiques (pages spéciales).
import { PARAMS } from '../../generated/params';
import type { UiOption, UiPage } from '../../generated/ui-layout';

/** `<kind>/<page>/<section>` → clé Orca (référence Tab.cpp, ORCA_SRC 2.4.1). */
export const DYNAMIC_OPTIONS: Record<string, string> = {
	'process/Others/Notes': 'notes', // TabPrint::build, Tab.cpp:2782
	'filament/Notes/Notes': 'filament_notes', // TabFilament::build, Tab.cpp:4237
	'machine/Basic information/Advanced': 'thumbnails', // TabPrinter::build_fff, Tab.cpp:4506
	'machine/Notes/Notes': 'printer_notes', // TabPrinter::build_fff, Tab.cpp:4717
	// G-code personnalisé (Phase 18 : n'était éditable nulle part) — Tab.cpp,
	// optgroups « … G-code » + get_option(<clé>).
	'process/Others/Change extrusion role G-code': 'process_change_extrusion_role_gcode', // 2768
	'process/Others/Post-processing Scripts': 'post_process', // 2775
	'filament/Advanced/Filament start G-code': 'filament_start_gcode', // 4137
	'filament/Advanced/Change extrusion role G-code': 'filament_change_extrusion_role_gcode', // 4148
	'filament/Advanced/Filament end G-code': 'filament_end_gcode', // 4159
	'machine/Machine G-code/File header G-code': 'file_start_gcode', // 4589
	'machine/Machine G-code/Machine start G-code': 'machine_start_gcode', // 4600
	'machine/Machine G-code/Machine end G-code': 'machine_end_gcode', // 4611
	'machine/Machine G-code/Printing by object G-code': 'printing_by_object_gcode', // 4622
	'machine/Machine G-code/Before layer change G-code': 'before_layer_change_gcode', // 4634
	'machine/Machine G-code/Layer change G-code': 'layer_change_gcode', // 4645
	'machine/Machine G-code/Timelapse G-code': 'time_lapse_gcode', // 4656
	'machine/Machine G-code/Clumping Detection G-code': 'wrapping_detection_gcode', // 4667
	'machine/Machine G-code/Change filament G-code': 'change_filament_gcode', // 4678
	'machine/Machine G-code/Change extrusion role G-code': 'change_extrusion_role_gcode', // 4689
	'machine/Machine G-code/Pause G-code': 'machine_pause_gcode', // 4700
	'machine/Machine G-code/Template Custom G-code': 'template_custom_gcode' // 4710
};

/**
 * Lignes à widget d'Orca (`create_line_with_widget`) absentes du layout généré,
 * ajoutées en fin de section : `<kind>/<page>/<section>` → clés.
 */
export const WIDGET_LINES: Record<string, string[]> = {
	// TabFilament::build, Tab.cpp:4191 — bouton « Ramming settings… » (RammingDialog).
	'filament/Multimaterial/Tool change parameters with single extruder MM printers': [
		'filament_ramming_parameters'
	]
};

/** Remplace les marqueurs dynamiques connus par leur clé (copie, layout intact). */
export function resolveDynamicOptions(layout: UiPage[]): UiPage[] {
	return layout.map((page) => ({
		...page,
		sections: page.sections.map((section) => {
			const path = `${page.kind}/${page.title}/${section.title}`;
			const options = section.options.map((option): UiOption => {
				if (typeof option === 'string') return option;
				const key = DYNAMIC_OPTIONS[path];
				return key && PARAMS[key] ? key : option;
			});
			for (const key of WIDGET_LINES[path] ?? [])
				if (PARAMS[key] && !options.includes(key)) options.push(key);
			return { ...section, options };
		})
	}));
}
