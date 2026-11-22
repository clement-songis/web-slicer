// Lignes composées d'Orca (Tab.cpp : `Line line = { L("Nozzle") }; line.append_option(…)`)
// à partir de `UI_LINES` (généré) : une section affiche ces options sous le
// libellé commun de la ligne (« Nozzle », « Cool Plate »…), comme
// OptionsGroup::append_line, sinon « First layer » / « Other layers » seraient
// ambigus. Fonctions pures, testées par ui-lines-p47b.test.ts.
import { UI_LINES, type PresetKind, type UiLine, type UiOption } from '../../generated/ui-layout';

/** Libellés d'option remplacés dans une ligne (Tab.cpp TabFilament::build :
 *  `chamber_temp_target_opt.opt.label = L("Target")`). */
export const LINE_OPTION_LABELS: Record<string, string> = {
	chamber_temperature: 'Target'
};

export type SectionRow =
	| { kind: 'option'; option: UiOption; index: number }
	| { kind: 'line'; line: UiLine; options: string[] };

export function sectionLines(kind: PresetKind, page: string, section: string): UiLine[] {
	return UI_LINES[`${kind}/${page}/${section}`] ?? [];
}

/** Une ligne n'apporte un libellé que si elle groupe plusieurs options, ou si
 *  son libellé diffère de celui de son unique option (`labelOf`). */
function meaningful(line: UiLine, labelOf: (key: string) => string | undefined): boolean {
	return line.options.length > 1 || labelOf(line.options[0]) !== line.label;
}

/** Découpe les options visibles d'une section en lignes simples et lignes
 *  composées (à la position de leur première option visible). */
export function sectionRows(
	options: readonly UiOption[],
	lines: readonly UiLine[],
	labelOf: (key: string) => string | undefined
): SectionRow[] {
	const owner = new Map<string, UiLine>();
	for (const line of lines) {
		if (!meaningful(line, labelOf)) continue;
		for (const key of line.options) if (!owner.has(key)) owner.set(key, line);
	}
	const visible = new Set(options.filter((o): o is string => typeof o === 'string'));
	const emitted = new Set<UiLine>();
	const rows: SectionRow[] = [];
	options.forEach((option, index) => {
		const line = typeof option === 'string' ? owner.get(option) : undefined;
		if (!line) {
			rows.push({ kind: 'option', option, index });
			return;
		}
		if (emitted.has(line)) return;
		emitted.add(line);
		rows.push({ kind: 'line', line, options: line.options.filter((key) => visible.has(key)) });
	});
	return rows;
}
