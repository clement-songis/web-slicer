<script lang="ts">
	import IconUnlink from '~icons/lucide/lock-open';
	import { dialogFocus } from '#lib/ui/dialog-focus';
	// Dialogue modal de réglages (Phase 14, T138) — parité des dialogues
	// OrcaSlicer « Paramètres de l'imprimante » / presets filament/process : les
	// mêmes onglets que l'édition inline (`SettingsTabs`, cadrés par type) mais
	// dans une fenêtre modale ouverte depuis le sélecteur. Présentational : les
	// valeurs sont liées, la persistance reste dans la page.
	import { t, tr } from '#lib/i18n';
	import type { PresetKind } from '../../generated/ui-layout';
	import type { DisplayMode } from './filter';
	import SettingsTabs from './SettingsTabs.svelte';
	import ModifiedBadge from '#lib/presets/ModifiedBadge.svelte';
	import { hasPresetChanges } from './editing';

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
		/** Onglet ouvert à l'ouverture (titre de page Orca). */
		initialPage?: string;
		/** Option mise en évidence à l'ouverture (notification de validation). */
		focusKey?: string | null;
		/** Valeurs enregistrées / système par type (marqueurs par option). */
		savedByKind?: Partial<Record<PresetKind, Record<string, unknown> | null>>;
		systemByKind?: Partial<Record<PresetKind, Record<string, unknown> | null>>;
		/** Preset édité (nom, origine) : barre de gestion comme l'en-tête des
		 *  onglets d'Orca (renommer / supprimer un preset utilisateur, comparer). */
		preset?: { id: string; name: string; origin: string; inherits?: string } | null;
		/** Renomme ; renvoie un message d'erreur ou null. */
		onrename?: (name: string) => Promise<string | null>;
		ondelete?: () => void;
		oncompare?: () => void;
		/** Détache du preset système parent (Orca : cadenas « Detach from system
		 *  preset ») ; `name` = nom de la copie pour un preset système. Renvoie
		 *  un message d'erreur ou null. */
		ondetach?: (name: string | null) => Promise<string | null>;
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
		saveAsError = null,
		initialPage,
		focusKey = null,
		savedByKind,
		systemByKind,
		preset = null,
		onrename,
		ondetach,
		ondelete,
		oncompare
	}: Props = $props();

	// Audit 40 A2 : en-tête marqué « (modifié) » quand les valeurs éditées
	// diffèrent du preset enregistré (Orca : titre de l'onglet « … (modified) »).
	const presetModified = $derived.by(() => {
		const saved = savedByKind?.[kind];
		return !!saved && hasPresetChanges(saved, values);
	});
	let renaming = $state<string | null>(null);
	let renameError = $state<string | null>(null);
	let confirmDelete = $state(false);
	// Détacher (Tab.cpp:2163) : visible si le parent est un preset système
	// (preset système lui-même, ou preset utilisateur qui en hérite).
	const canDetach = $derived(
		!!preset && !!ondetach && (preset.origin !== 'user' || !!preset.inherits)
	);
	let detaching = $state<string | null>(null);
	let detachError = $state<string | null>(null);
	let detachBusy = $state(false);
	function startDetach() {
		detachError = null;
		detaching =
			preset && preset.origin !== 'user' ? tr('{name} (detached)', { name: preset.name }) : '';
	}
	async function confirmDetach() {
		if (detaching === null || !ondetach || !preset) return;
		const system = preset.origin !== 'user';
		const name = detaching.trim();
		if (system && !name) {
			detachError = tr('name required');
			return;
		}
		detachBusy = true;
		detachError = await ondetach(system ? name : null);
		detachBusy = false;
		if (!detachError) detaching = null;
	}
	$effect(() => {
		if (!open) {
			renaming = null;
			renameError = null;
			confirmDelete = false;
		}
	});
	async function commitRename() {
		if (renaming === null || !onrename) return;
		const name = renaming.trim();
		if (!name || name === preset?.name) {
			renaming = null;
			return;
		}
		renameError = await onrename(name);
		if (!renameError) renaming = null;
	}

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
			<div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
				<div class="flex min-w-0 flex-col">
					<h2 class="ws-dialog-title">{title}</h2>
					{#if preset}
						<div
							class="flex min-w-0 items-center gap-2 text-xs text-content-muted"
							data-testid="preset-bar"
						>
							{#if renaming !== null}
								<input
									class="ws-field ws-field-sm w-64 {renameError ? 'border-danger' : ''}"
									bind:value={renaming}
									aria-label={$t('New preset name')}
									aria-invalid={!!renameError}
									data-testid="preset-rename-input"
									onkeydown={(e) => {
										if (e.key === 'Enter') void commitRename();
										if (e.key === 'Escape') {
											e.stopPropagation();
											renaming = null;
										}
									}}
								/>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-primary"
									onclick={commitRename}
								>
									{$t('Rename')}
								</button>
								{#if renameError}<span class="text-danger" role="alert">{renameError}</span>{/if}
							{:else}
								<!-- Le nom passe avant « hérite de … » : seul ce dernier se tronque
								     (« Mon … » tronqué avec 4 boutons d'en-tête, phase 27). -->
								<span
									class="max-w-full shrink-0 truncate font-medium text-content"
									data-testid="preset-name"
									title={preset.name}
								>
									{preset.name}
								</span>
								<span class="ws-badge h-4 shrink-0 px-1.5">
									{preset.origin === 'user' ? $t('User') : $t('System')}
								</span>
								{#if presetModified}<ModifiedBadge testid="preset-modified" />{/if}
								{#if preset.inherits}
									<span
										class="min-w-0 truncate text-content-subtle"
										title={$t('inherits from {name}', { name: preset.inherits })}
									>
										{$t('inherits from {name}', { name: preset.inherits })}
									</span>
								{/if}
							{/if}
						</div>
					{/if}
				</div>
				<div class="flex shrink-0 items-center gap-2">
					{#if canDetach && detaching === null}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="preset-detach"
							title={$t(
								'Make this preset standalone: it will no longer follow the updates of “{name}”',
								{
									name: (preset?.origin === 'user' ? preset?.inherits : preset?.name) ?? ''
								}
							)}
							onclick={startDetach}
						>
							<IconUnlink class="mr-1 inline h-3.5 w-3.5 align-[-2px]" aria-hidden="true" />{$t(
								'Detach…'
							)}
						</button>
					{/if}
					{#if preset && oncompare}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="preset-compare"
							onclick={oncompare}
						>
							{$t('Compare…')}
						</button>
					{/if}
					{#if preset?.origin === 'user' && onrename && renaming === null}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							data-testid="preset-rename"
							onclick={() => {
								renaming = preset?.name ?? '';
								renameError = null;
							}}
						>
							{$t('Rename')}
						</button>
					{/if}
					{#if preset?.origin === 'user' && ondelete}
						{#if confirmDelete}
							<span class="text-xs text-content-muted">
								{$t('Delete “{name}”?', { name: preset.name })}
							</span>
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-danger"
								data-testid="preset-delete-confirm"
								onclick={() => {
									confirmDelete = false;
									ondelete?.();
								}}
							>
								{$t('Delete')}
							</button>
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-ghost"
								onclick={() => (confirmDelete = false)}
							>
								{$t('Cancel')}
							</button>
						{:else}
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-ghost text-danger hover:bg-danger-soft hover:text-danger-content"
								data-testid="preset-delete"
								onclick={() => (confirmDelete = true)}
							>
								{$t('Delete')}
							</button>
						{/if}
					{/if}
					<button type="button" class="ws-btn ws-btn-secondary" onclick={onClose}>
						{$t('Close')}
					</button>
				</div>
			</div>
			{#if detaching !== null && preset}
				<div
					class="text-warning-text mx-4 mt-2 flex flex-col gap-2 rounded-md border border-warning/40 bg-warning-soft px-3 py-2 text-sm"
					role="alertdialog"
					aria-label={$t('Detach preset')}
					data-testid="preset-detach-panel"
				>
					<p>
						{#if preset.origin !== 'user'}
							{$t(
								'A standalone copy of “{name}” will be created in your presets. It keeps all the current values but will no longer follow the updates of the system preset.',
								{ name: preset.name }
							)}
						{:else}
							{$t(
								'“{name}” will become standalone: the values inherited from “{parent}” are copied into it, and it will no longer follow its updates.',
								{ name: preset.name, parent: preset.inherits ?? '' }
							)}
						{/if}
						{$t('Unsaved changes are not included. This cannot be undone.')}
					</p>
					<div class="flex flex-wrap items-center gap-2">
						{#if preset.origin !== 'user'}
							<label class="flex items-center gap-2 text-xs">
								{$t('Copy name')}
								<input
									class="ws-field ws-field-sm w-64"
									data-testid="preset-detach-name"
									bind:value={detaching}
									onkeydown={(e) => e.key === 'Enter' && void confirmDetach()}
								/>
							</label>
						{/if}
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-primary"
							data-testid="preset-detach-confirm"
							disabled={detachBusy}
							aria-busy={detachBusy}
							onclick={confirmDetach}
						>
							{$t('Detach')}
						</button>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-ghost"
							onclick={() => (detaching = null)}
						>
							{$t('Cancel')}
						</button>
						{#if detachError}<span class="text-danger" role="alert">{detachError}</span>{/if}
					</div>
				</div>
			{/if}

			<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
				<!-- Tant que `/resolved` n'a pas répondu (jusqu'à plusieurs secondes pour
				     les ~7000 filaments système), mode et onglets restent utilisables
				     mais aucun champ n'est affiché (pas d'état neutre pris pour le preset). -->
				<SettingsTabs
					{kind}
					bind:mode
					bind:values
					{valuesByKind}
					{initialPage}
					{focusKey}
					{savedByKind}
					{systemByKind}
					loading={Object.keys(values).length === 0}
				/>
			</div>

			{#if onsave}
				<div class="flex items-center justify-end gap-3 border-t border-border px-4 py-3">
					{#if saveAsName !== null}
						<label class="flex min-w-0 flex-1 flex-col gap-0.5 text-xs text-content-muted">
							<span>{$t('System preset: save under the name')}</span>
							<input
								class="ws-field w-full {saveAsError ? 'border-danger' : ''}"
								bind:value={saveAsName}
								data-testid="settings-save-as-name"
								aria-invalid={!!saveAsError}
								aria-describedby="settings-save-as-error"
							/>
							{#if saveAsError}
								<span id="settings-save-as-error" class="text-danger" role="alert">
									{saveAsError}
								</span>
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
						{$t('Save')}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
