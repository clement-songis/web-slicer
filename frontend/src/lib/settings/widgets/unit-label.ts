// Unité des champs de réglage, réactive à la langue et à la devise (phase 38).
import { derived } from 'svelte/store';
import { t } from '#lib/i18n';
import { appPreferences } from '#lib/settings/preferences';
import { displayUnit } from './types';

/** `$unitLabel(def.sidetext)` : unité traduite, devise des préférences pour « money/… ». */
export const unitLabel = derived(
	[t, appPreferences],
	([translate, prefs]) =>
		(sidetext: string | null | undefined) =>
			displayUnit(sidetext, translate, prefs.currency)
);
