// Point d'entrée du module de réglages (T042) : composants de rendu générique
// et cœur de filtrage mode/recherche.

export { default as SettingsTabs } from './SettingsTabs.svelte';
export { default as OptionLine } from './OptionLine.svelte';
export {
	filterLayout,
	optionVisible,
	modeAllows,
	matchesQuery,
	MODE_RANK,
	type DisplayMode
} from './filter';
export { SettingsStore, type SettingValues } from './store';
export { validateValue, type Validation } from './validate';
