// Traductions des réglages chargées à la demande (perf) : ≈1100 libellés,
// infobulles et valeurs d'énumération issus de OrcaSlicer_fr.po
// (scripts/gen-settings-i18n.mjs). Importées à l'ouverture du dialogue de
// réglages au lieu d'alourdir le chunk initial de chaque page.
import { registerTranslations, type Locale } from '#lib/i18n';
import { SETTINGS_FR_EXTRA } from './i18n-extra';

const loaders: Partial<Record<Locale, () => Promise<{ default: Record<string, string> }>>> = {
	fr: () => import('./i18n/fr.json')
};

const pending = new Map<Locale, Promise<void>>();

/** Charge (une fois) le dictionnaire des réglages pour `loc` ; l'anglais est l'identité. */
export function loadSettingsTranslations(loc: Locale): Promise<void> {
	const load = loaders[loc];
	if (!load) return Promise.resolve();
	let p = pending.get(loc);
	if (!p) {
		p = load()
			.then((mod) =>
				registerTranslations(
					loc,
					loc === 'fr' ? { ...mod.default, ...SETTINGS_FR_EXTRA } : mod.default
				)
			)
			.catch((e) => {
				pending.delete(loc); // nouvel essai à la prochaine ouverture
				throw e;
			});
		pending.set(loc, p);
	}
	return p;
}
