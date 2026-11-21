// Type de débit de buse par extrudeur (Orca Sidebar ExtruderGroup :
// « Débit » Standard / Haut débit, TabPrinter::set_extruder_volume_type) pour
// les imprimantes dont `extruder_variant_list` propose une variante « High
// Flow » (Bambu Lab…). La valeur choisie est `nozzle_volume_type[i]` de la
// config machine ; le moteur choisit alors la variante des réglages filament
// (filament_extruder_variant) de chaque extrudeur. Pur.

import { tr } from '$lib/i18n';

export type NozzleFlow = 'Standard' | 'High Flow';

/** Libellés traduits à la lecture (Orca fr : « Standard », « Haut débit »). */
export const NOZZLE_FLOW_LABELS: Readonly<Record<NozzleFlow, string>> = {
	get Standard() {
		return tr('Standard');
	},
	get 'High Flow'() {
		return tr('High Flow');
	}
};

/** Sélecteur d'un extrudeur : types proposés et type actuel. */
export interface NozzleFlowChoice {
	index: number;
	options: NozzleFlow[];
	value: NozzleFlow;
}

const asList = (v: unknown): string[] =>
	Array.isArray(v) ? v.map((x) => String(x ?? '')) : typeof v === 'string' && v ? v.split(';') : [];

/**
 * Sélecteurs par extrudeur, ou `[]` si l'imprimante n'a pas de variante de
 * débit (aucune liste ne contient « High Flow »). Les types proposés sont ceux
 * de la liste de variantes de l'extrudeur (« Direct Drive Standard,Direct
 * Drive High Flow »).
 */
export function nozzleFlowChoices(machine: Readonly<Record<string, unknown>>): NozzleFlowChoice[] {
	const lists = asList(machine.extruder_variant_list);
	if (!lists.some((l) => /High Flow/.test(l))) return [];
	const count = Math.max(lists.length, asList(machine.nozzle_diameter).length, 1);
	const current = asList(machine.nozzle_volume_type);
	return Array.from({ length: count }, (_, index) => {
		const variants = (lists[index] ?? lists[0] ?? '').split(',');
		const options = (['Standard', 'High Flow'] as const).filter((t) =>
			variants.some((v) => v.trim().endsWith(t))
		);
		const raw = current[index] ?? current[0];
		const value: NozzleFlow = raw === 'High Flow' || raw === '1' ? 'High Flow' : 'Standard';
		return { index, options: options.length ? [...options] : ['Standard'], value };
	});
}

/** Nouveau vecteur `nozzle_volume_type` avec l'extrudeur `index` réglé. */
export function setNozzleFlow(
	machine: Readonly<Record<string, unknown>>,
	index: number,
	flow: NozzleFlow
): string[] {
	const count = Math.max(nozzleFlowChoices(machine).length, index + 1);
	const current = asList(machine.nozzle_volume_type);
	return Array.from({ length: count }, (_, i) =>
		i === index ? flow : current[i] === 'High Flow' || current[i] === '1' ? 'High Flow' : 'Standard'
	);
}
