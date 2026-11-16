// Préférences applicatives (menu « Preferences », sous-ensemble Orca faisable
// côté web) : thème, langue, unités d'affichage, avertissement avant fermeture.
// Persistées dans localStorage ; le thème et la langue délèguent à leurs
// modules (`$lib/theme`, `$lib/i18n`) qui gèrent leur propre stockage.

export type Units = 'mm' | 'in';

export interface AppPreferences {
	units: Units;
	/** Avertir avant de quitter la page (Orca : « Show warning before close »). */
	confirmClose: boolean;
	/** Panneau latéral de l'éditeur replié (Shift+Tab, groupe Plater). */
	sidebarCollapsed: boolean;
	/** Devise des coûts (Orca : `filament_cost` en « money/kg », sans devise :
	 *  l'unité monétaire est celle saisie dans le filament ; on l'affiche). */
	currency: string;
}

export const PREFERENCES_KEY = 'web-slicer.preferences';

export const DEFAULT_PREFERENCES: AppPreferences = {
	units: 'mm',
	confirmClose: true,
	sidebarCollapsed: false,
	currency: '€'
};

/** Lecture tolérante (valeurs inconnues → défauts). */
export function parsePreferences(raw: string | null): AppPreferences {
	try {
		const value = raw ? (JSON.parse(raw) as Partial<AppPreferences>) : {};
		return {
			units: value.units === 'in' ? 'in' : 'mm',
			confirmClose:
				typeof value.confirmClose === 'boolean'
					? value.confirmClose
					: DEFAULT_PREFERENCES.confirmClose,
			sidebarCollapsed:
				typeof value.sidebarCollapsed === 'boolean'
					? value.sidebarCollapsed
					: DEFAULT_PREFERENCES.sidebarCollapsed,
			currency:
				typeof value.currency === 'string' && value.currency.trim()
					? value.currency.trim().slice(0, 4)
					: DEFAULT_PREFERENCES.currency
		};
	} catch {
		return { ...DEFAULT_PREFERENCES };
	}
}

export function loadPreferences(): AppPreferences {
	try {
		return parsePreferences(localStorage.getItem(PREFERENCES_KEY));
	} catch {
		return { ...DEFAULT_PREFERENCES };
	}
}

export function savePreferences(prefs: AppPreferences): void {
	try {
		localStorage.setItem(PREFERENCES_KEY, JSON.stringify(prefs));
	} catch {
		// Stockage indisponible : préférence valable pour la session seulement.
	}
}

/** Montant formaté avec la devise (« 0,08 € »). */
export function formatMoney(value: number, currency: string): string {
	const n = value.toLocaleString('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	return currency ? `${n} ${currency}` : n;
}

/** Longueur (mm) formatée selon l'unité d'affichage. */
export function formatLength(mm: number, units: Units, digits = 2): string {
	return units === 'in' ? `${(mm / 25.4).toFixed(digits)} in` : `${mm.toFixed(digits)} mm`;
}
