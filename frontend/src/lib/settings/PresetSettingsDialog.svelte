<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Dialogue modal de réglages (Phase 14, T138) — parité des dialogues
	// OrcaSlicer « Paramètres de l'imprimante » / presets filament/process : les
	// mêmes onglets que l'édition inline (`SettingsTabs`, cadrés par type) mais
	// dans une fenêtre modale ouverte depuis le sélecteur. Présentational : les
	// valeurs sont liées, la persistance reste dans la page.
	import { t } from '$lib/i18n';
	import type { PresetKind } from '../../generated/ui-layout';
	import type { DisplayMode } from './filter';
	import SettingsTabs from './SettingsTabs.svelte';

	interface Props {
		open?: boolean;
		/** Titre du dialogue (ex. « Paramètres de l'imprimante »). */
		title: string;
		/** Type de preset (cadre les onglets). */
		kind: PresetKind;
		/** Mode d'affichage (liable). */
		mode?: DisplayMode;
		/** Valeurs effectives éditées (liables). */
		values?: Record<string, unknown>;
		/** Brouillons des autres presets pour les dépendances inter-presets. */
		valuesByKind?: Record<PresetKind, Record<string, unknown>>;
		/** Fermeture (croix, fond, Échap). */
		onClose: () => void;
		/** Enregistrement (facultatif) ; absent → dialogue en lecture. */
		onsave?: () => void;
		/** Désactive « Enregistrer » (ex. preset système non éditable). */
		saveDisabled?: boolean;
		/** Info affichée à côté de « Enregistrer » (ex. « dérivez d'abord »). */
		saveHint?: string;
		/** Preset système modifié : nom de la copie utilisateur à créer (liable),
		 *  proposé puis éditable, comme le dialogue « Enregistrer le preset » d'Orca.
		 *  `null` → pas de champ (preset utilisateur mis à jour en place). */
		saveAsName?: string | null;
		/** Erreur de validation du nom (vide, doublon). */
		saveAsError?: string | null;
	}

	let {
		open = false,
		title,
		kind,
		mode = $bindable('simple'),
		values = $bindable({}),
		valuesByKind,
		onClose,
		onsave,
		saveDisabled = false,
		saveHint,
		saveAsName = $bindable(null),
		saveAsError = null
	}: Props = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}

	function onDialogKeydown(event: KeyboardEvent) {
		// Les autres touches restent dans la modale ; Échap doit atteindre
		// le gestionnaire global qui la ferme réellement.
		if (event.key !== 'Escape') event.stopPropagation();
	}
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="ws-dialog flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden"
			role="dialog"
			data-testid="settings-dialog"
			aria-modal="true"
			aria-label={title}
			onclick={(e) => e.stopPropagation()}
			onkeydown={onDialogKeydown}
			tabindex="-1"
			use:dialogFocus
		>
			<div class="flex items-center justify-between border-b border-border px-4 py-3">
				<h2 class="ws-dialog-title">{title}</h2>
				<button type="button" class="ws-btn ws-btn-secondary" onclick={onClose}>
					{$t('Close')}
				</button>
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
				<!-- Tant que `/resolved` n'a pas répondu (jusqu'à plusieurs secondes pour
				     les ~7000 filaments système), mode et onglets restent utilisables
				     mais aucun champ n'est affiché (pas d'état neutre pris pour le preset). -->
				<SettingsTabs
					{kind}
					bind:mode
					bind:values
					{valuesByKind}
					loading={Object.keys(values).length === 0}
				/>
			</div>

			{#if onsave}
				<div class="flex items-center justify-end gap-3 border-t border-border px-4 py-3">
					{#if saveAsName !== null}
						<label class="flex min-w-0 flex-1 flex-col gap-0.5 text-xs text-content-muted">
							<span>Preset système : enregistrer sous le nom</span>
							<input
								class="ws-field w-full {saveAsError ? 'border-danger' : ''}"
								bind:value={saveAsName}
								data-testid="settings-save-as-name"
								aria-invalid={!!saveAsError}
								aria-describedby="settings-save-as-error"
							/>
							{#if saveAsError}
								<span id="settings-save-as-error" class="text-danger" role="alert"
									>{saveAsError}</span
								>
							{/if}
						</label>
					{/if}
					{#if saveHint}
						<span class="text-xs text-content-subtle">{saveHint}</span>
					{/if}
					<button
						type="button"
						data-testid="settings-apply"
						class="ws-btn ws-btn-primary"
						disabled={saveDisabled || !!saveAsError}
						onclick={onsave}
					>
						Enregistrer
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
