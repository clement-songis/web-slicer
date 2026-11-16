<script lang="ts">
	// Dialogue « Preferences » (menu Fichier/Application) : thème, langue,
	// unités, avertissement avant fermeture. Applique immédiatement.
	import { THEME_PREFS, themePref, setTheme, type ThemePref } from '$lib/theme';
	import { LOCALES, locale, setLocale, type Locale } from '$lib/i18n';
	import { savePreferences, type AppPreferences, type Units } from './preferences';
	import { dialogFocus } from '$lib/ui/dialog-focus';

	interface Props {
		open: boolean;
		prefs: AppPreferences;
		onchange: (prefs: AppPreferences) => void;
		onclose: () => void;
	}

	let { open, prefs, onchange, onclose }: Props = $props();

	const themeLabels: Record<ThemePref, string> = {
		light: 'Clair',
		dark: 'Sombre',
		system: 'Système'
	};

	function update(patch: Partial<AppPreferences>) {
		const next = { ...prefs, ...patch };
		savePreferences(next);
		onchange(next);
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim"
		role="presentation"
		onclick={onclose}
	>
		<div
			class="ws-dialog w-96 p-4"
			role="dialog"
			aria-modal="true"
			aria-label="Préférences"
			data-testid="preferences-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onclose()}
		>
			<h2 class="ws-dialog-title mb-3">Préférences</h2>
			<div class="flex flex-col gap-3 text-sm">
				<label class="flex items-center justify-between gap-2">
					<span>Thème</span>
					<select
						data-testid="pref-theme"
						class="ws-field ws-field-sm"
						value={$themePref}
						onchange={(e) => setTheme(e.currentTarget.value as ThemePref)}
					>
						{#each THEME_PREFS as pref (pref)}
							<option value={pref}>{themeLabels[pref]}</option>
						{/each}
					</select>
				</label>
				<label class="flex items-center justify-between gap-2">
					<span>Langue</span>
					<select
						data-testid="pref-language"
						class="ws-field ws-field-sm"
						value={$locale}
						onchange={(e) => setLocale(e.currentTarget.value as Locale)}
					>
						{#each LOCALES as code (code)}
							<option value={code}>{code}</option>
						{/each}
					</select>
				</label>
				<label class="flex items-center justify-between gap-2">
					<span>Unités</span>
					<select
						data-testid="pref-units"
						class="ws-field ws-field-sm"
						value={prefs.units}
						onchange={(e) => update({ units: e.currentTarget.value as Units })}
					>
						<option value="mm">Millimètres</option>
						<option value="in">Pouces</option>
					</select>
				</label>
				<label class="flex items-center justify-between gap-2">
					<span>Devise des coûts</span>
					<input
						data-testid="pref-currency"
						class="ws-field ws-field-sm w-20 text-center"
						maxlength="4"
						value={prefs.currency}
						onchange={(e) => update({ currency: e.currentTarget.value.trim() || '€' })}
						aria-describedby="pref-currency-hint"
					/>
				</label>
				<p id="pref-currency-hint" class="-mt-2 text-xs text-content-subtle">
					Unité du « Coût du filament » (par kg) saisi dans les réglages du filament.
				</p>
				<label class="flex items-center justify-between gap-2">
					<span>Avertir avant de fermer</span>
					<input
						type="checkbox"
						data-testid="pref-confirm-close"
						checked={prefs.confirmClose}
						onchange={(e) => update({ confirmClose: e.currentTarget.checked })}
					/>
				</label>
			</div>
			<div class="mt-4 flex justify-end">
				<button type="button" class="ws-btn ws-btn-primary" onclick={onclose}>Fermer</button>
			</div>
		</div>
	</div>
{/if}
