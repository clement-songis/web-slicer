<script lang="ts">
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
		/** Fermeture (croix, fond, Échap). */
		onClose: () => void;
		/** Enregistrement (facultatif) ; absent → dialogue en lecture. */
		onsave?: () => void;
		/** Désactive « Enregistrer » (ex. preset système non éditable). */
		saveDisabled?: boolean;
		/** Info affichée à côté de « Enregistrer » (ex. « dérivez d'abord »). */
		saveHint?: string;
	}

	let {
		open = false,
		title,
		kind,
		mode = $bindable('simple'),
		values = $bindable({}),
		onClose,
		onsave,
		saveDisabled = false,
		saveHint
	}: Props = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-lg border border-border bg-surface-raised text-content shadow-xl"
			role="dialog"
			aria-modal="true"
			aria-label={title}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			tabindex="-1"
		>
			<div class="flex items-center justify-between border-b border-border px-4 py-3">
				<h2 class="text-lg font-semibold text-content">{title}</h2>
				<button
					type="button"
					class="rounded bg-overlay px-2 py-1 text-sm hover:bg-border-strong"
					onclick={onClose}
				>
					{$t('Close')}
				</button>
			</div>

			<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
				<SettingsTabs {kind} bind:mode bind:values />
			</div>

			{#if onsave}
				<div class="flex items-center justify-end gap-3 border-t border-border px-4 py-3">
					{#if saveHint}
						<span class="text-xs text-content-subtle">{saveHint}</span>
					{/if}
					<button
						type="button"
						class="rounded bg-primary px-3 py-1 text-sm font-medium text-primary-content disabled:opacity-50"
						disabled={saveDisabled}
						onclick={onsave}
					>
						{$t('Save')}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
