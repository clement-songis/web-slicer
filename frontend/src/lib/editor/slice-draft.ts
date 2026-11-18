// Brouillon envoyé au tranchage (lot 21, `SliceDraft`) : Orca tranche avec
// le preset **édité**, pas la version enregistrée. Seules les valeurs qui
// diffèrent du preset résolu sont envoyées (mêmes règles que « modifié »).
import { changedPresetValues } from '$lib/settings/editing';
import type { SliceDraft } from '$lib/api/types';

type Values = Record<string, unknown>;

export interface DraftInput {
	process: { resolved: Values; current: Values };
	printer: { resolved: Values; current: Values };
	/** Filament édité : un seul slot à la fois (celui du panneau de réglages). */
	filament: { slot: number; count: number; resolved: Values; current: Values };
}

/** Brouillon, ou `undefined` si rien n'est modifié. */
export function buildSliceDraft(input: DraftInput): SliceDraft | undefined {
	const process = changedPresetValues(input.process.resolved, input.process.current);
	const printer = changedPresetValues(input.printer.resolved, input.printer.current);
	const filament = changedPresetValues(input.filament.resolved, input.filament.current);
	const draft: SliceDraft = {};
	if (Object.keys(process).length) draft.process = process;
	if (Object.keys(printer).length) draft.printer = printer;
	if (Object.keys(filament).length && input.filament.count > 0) {
		const slot = Math.min(Math.max(0, input.filament.slot), input.filament.count - 1);
		draft.filaments = Array.from({ length: input.filament.count }, (_, i) =>
			i === slot ? filament : null
		);
	}
	return Object.keys(draft).length ? draft : undefined;
}
