<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Dialogue « Réglages du plateau » (parité Orca PlateSettingsDialog, ouvert
	// par l'icône du plateau) : type de plaque, séquence d'impression, séquence
	// des couches, vase spirale. Brouillon local appliqué par « Enregistrer » ;
	// une valeur vide = suivre le profil (pas de surcharge du plateau).
	import { PARAMS } from '../../generated/params';
	import { t } from '$lib/i18n';
	import { modeAllows, type DisplayMode } from './filter';
	import { isOpenEnum } from './widgets/types';

	interface Props {
		open?: boolean;
		/** Surcharges actuelles du plateau (clé → valeur Orca). */
		settings?: Record<string, unknown>;
		mode?: DisplayMode;
		onClose: () => void;
		/** Applique une surcharge (`undefined` = retour au profil). */
		onsetting: (key: string, value: unknown) => void;
	}

	let {
		open = false,
		settings = {},
		mode = $bindable('simple'),
		onClose,
		onsetting
	}: Props = $props();

	/** Options du dialogue Orca, dans son ordre. */
	const PLATE_KEYS = [
		'curr_bed_type',
		'print_sequence',
		'first_layer_sequence_choice',
		'other_layers_sequence_choice',
		'spiral_mode'
	] as const;
	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert', 'develop'];
	/** Libellés FR des modes, identiques au dialogue de réglages (SettingsTabs). */
	const MODE_LABELS: Record<DisplayMode, string> = {
		simple: 'Simple',
		advanced: 'Avancé',
		expert: 'Expert',
		develop: 'Développeur'
	};

	let draft = $state<Record<string, string>>({});
	// Ouverture : brouillon = surcharges actuelles (texte Orca).
	$effect(() => {
		if (!open) return;
		const next: Record<string, string> = {};
		for (const key of PLATE_KEYS) {
			const v = settings?.[key];
			next[key] =
				v === undefined || v === null ? '' : typeof v === 'boolean' ? (v ? '1' : '0') : String(v);
		}
		draft = next;
	});

	const visible = $derived(
		PLATE_KEYS.filter((key) => PARAMS[key] && modeAllows(PARAMS[key].mode, mode))
	);

	function apply() {
		for (const key of PLATE_KEYS) {
			const before = settings?.[key];
			const was = before === undefined || before === null ? '' : String(before);
			if (draft[key] !== was) onsetting(key, draft[key] === '' ? undefined : draft[key]);
		}
		onClose();
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="ws-dialog flex w-full max-w-lg flex-col gap-3 p-4"
			role="dialog"
			aria-modal="true"
			aria-label="Réglages du plateau"
			data-testid="settings-dialog"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onClose()}
			use:dialogFocus={onClose}
		>
			<h2 class="ws-dialog-title">Réglages du plateau</h2>
			<div class="ws-seg flex w-full" role="group" aria-label="Mode d'affichage">
				{#each MODES as m (m)}
					<button
						type="button"
						data-testid={`settings-mode-${m}`}
						aria-pressed={mode === m}
						class="ws-seg-item flex-1 {mode === m ? 'ws-seg-on' : ''}"
						onclick={() => (mode = m)}>{MODE_LABELS[m]}</button
					>
				{/each}
			</div>
			<div class="flex flex-col divide-y divide-border">
				{#each visible as key (key)}
					{@const def = PARAMS[key]}
					<label class="flex items-center justify-between gap-2 py-1.5 text-sm" data-param={key}>
						<span class="text-content-muted">{$t(def.label)}</span>
						{#if def.type === 'coBool'}
							<select class="ws-field ws-field-sm w-48 shrink-0" bind:value={draft[key]}>
								<option value="">Profil</option>
								<option value="1">Oui</option>
								<option value="0">Non</option>
							</select>
						{:else if isOpenEnum(def)}
							<input
								type="text"
								class="ws-field ws-field-sm w-48 shrink-0"
								placeholder="Profil"
								bind:value={draft[key]}
							/>
						{:else}
							<select class="ws-field ws-field-sm w-48 shrink-0" bind:value={draft[key]}>
								<option value="">Profil</option>
								{#each def.enumValues as v, i (v)}
									<option value={v}>{$t(def.enumLabels[i] || v)}</option>
								{/each}
							</select>
						{/if}
					</label>
					{#if key === 'print_sequence' && draft[key] === 'by object'}
						<p class="pb-1.5 text-xs text-content-subtle" data-testid="plate-sequence-hint">
							Objet par objet : l'ordre de la liste d'objets est l'ordre d'impression (glissez un
							objet sur un autre pour le changer).
						</p>
					{/if}
				{/each}
			</div>
			<div class="flex justify-end gap-2">
				<button type="button" class="ws-btn ws-btn-secondary" onclick={onClose}
					>{$t('Close')}</button
				>
				<button
					type="button"
					data-testid="settings-apply"
					class="ws-btn ws-btn-primary"
					onclick={apply}>Enregistrer</button
				>
			</div>
		</div>
	</div>
{/if}
