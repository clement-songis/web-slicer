// Brouillon envoyé au tranchage (lot 21, `SliceDraft`) : Orca tranche avec
// le preset **édité**, pas la version enregistrée. Seules les valeurs qui
// diffèrent du preset résolu sont envoyées (mêmes règles que « modifié »).
import { changedPresetValues } from '$lib/settings/editing';
import type { SliceDraft } from '$lib/api/types';
import { compactOverrides, overridesOf, type ActivePresets } from './presets';

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

/** Brouillon d'un type de preset tel que chargé dans le panneau de réglages. */
export interface LiveDraft {
	/** Preset dont la base `/resolved` alimente `current` (null : pas chargé). */
	loadedId: string | null;
	resolved: Values;
	current: Values;
}

export interface OverridesInput {
	/** Sélection courante (ids des presets actifs). */
	active: ActivePresets;
	/**
	 * Surcharges enregistrées pas encore chargées dans un brouillon, avec les
	 * ids des presets auxquels elles s'appliquent (`overrides` du projet ouvert,
	 * slots filament non affichés) : ignorées si le preset du type a changé.
	 */
	pending: ActivePresets;
	process: LiveDraft;
	printer: LiveDraft;
	/** Filament édité : un seul slot (`slot`, null = aucun) à la fois. */
	filament: LiveDraft & { slot: number | null };
}

/**
 * Lot 40 (D1) — surcharges du projet (`active_presets.overrides`) : pour chaque
 * type, le brouillon chargé (valeurs qui diffèrent du preset, comme
 * `buildSliceDraft`) s'il porte sur le preset actif, sinon les surcharges
 * enregistrées encore en attente pour ce preset. Sert à la sauvegarde, au
 * tranchage et à la validation (`draft`) : même config effective partout.
 */
export function buildProjectOverrides(input: OverridesInput): SliceDraft | undefined {
	const { active, pending } = input;
	const live = (d: LiveDraft, id: string | null) =>
		d.loadedId !== null && d.loadedId === id ? changedPresetValues(d.resolved, d.current) : null;
	const process =
		live(input.process, active.process) ??
		(pending.process === active.process ? overridesOf(pending.overrides, 'process') : null);
	const printer =
		live(input.printer, active.printer) ??
		(pending.printer === active.printer ? overridesOf(pending.overrides, 'machine') : null);
	const filaments = active.filaments.map((id, i) => {
		if (input.filament.slot === i && input.filament.loadedId === id)
			return changedPresetValues(input.filament.resolved, input.filament.current);
		return pending.filaments[i] === id ? overridesOf(pending.overrides, 'filament', i) : null;
	});
	return compactOverrides({ process, printer, filaments });
}
