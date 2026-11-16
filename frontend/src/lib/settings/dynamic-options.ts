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
	'machine/Notes/Notes': 'printer_notes' // TabPrinter::build_fff, Tab.cpp:4717
};

/** Remplace les marqueurs dynamiques connus par leur clé (copie, layout intact). */
export function resolveDynamicOptions(layout: UiPage[]): UiPage[] {
	return layout.map((page) => ({
		...page,
		sections: page.sections.map((section) => ({
			...section,
			options: section.options.map((option): UiOption => {
				if (typeof option === 'string') return option;
				const key = DYNAMIC_OPTIONS[`${page.kind}/${page.title}/${section.title}`];
				return key && PARAMS[key] ? key : option;
			})
		}))
	}));
}
