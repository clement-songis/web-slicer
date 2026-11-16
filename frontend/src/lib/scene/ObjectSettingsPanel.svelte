<script lang="ts">
	// Réglages par objet / par volume (parité Orca « Add settings », panneau
	// des réglages d'objet) : liste des surcharges, ajout par recherche parmi les
	// clés autorisées (SettingsFactory::get_options), suppression. Purement
	// présentationnel : `onchange` reçoit le nouvel objet de surcharges.
	import { PARAMS } from '../../generated/params';
	import OptionLine from '$lib/settings/OptionLine.svelte';
	import type { DisplayMode } from '$lib/settings/filter';
	import { locale, t } from '$lib/i18n';
	import { loadSettingsTranslations } from '$lib/settings/i18n';
	import IconX from '~icons/lucide/x';
	import { settingsClipboard, pasteSettings } from './settings-clipboard.svelte';
	import {
		addOverride,
		removeOverride,
		searchOverridable,
		overriddenKeys,
		groupByCategory
	} from './object-settings';

	interface Props {
		/** Titre affiché (nom de l'objet ou du volume). */
		title: string;
		/** Surcharges actuelles (clé registre → valeur). */
		settings: Record<string, unknown>;
		/** Volume (pièce/modificateur) : clés de région uniquement. */
		isPart?: boolean;
		/** Mode d'affichage des réglages (simple/avancé/expert/develop). */
		mode?: DisplayMode;
		/** Valeurs du process actif : valeur initiale d'une nouvelle surcharge. */
		base?: Record<string, unknown>;
		/** Nombre de filaments du projet (masque « Extruders » s'il n'y en a qu'un). */
		filamentCount?: number;
		/** Ouvre directement la recherche (menu « Ajouter des réglages »). */
		adding?: boolean;
		onchange: (settings: Record<string, unknown>) => void;
		testid?: string;
	}

	let {
		title,
		settings,
		isPart = false,
		mode = 'advanced',
		base = {},
		filamentCount = 1,
		adding = $bindable(false),
		onchange,
		testid = 'object-settings-panel'
	}: Props = $props();

	// Libellés des réglages (OrcaSlicer_fr.po) : même dictionnaire paresseux que
	// le dialogue de réglages, sinon la liste « + Réglage » reste en anglais.
	$effect(() => {
		loadSettingsTranslations($locale).catch(() => {});
	});

	let query = $state('');
	const keys = $derived(overriddenKeys(settings));
	// Sans recherche : tous les réglages ajoutables, groupés par catégorie
	// (« Ajouter des réglages » d'Orca, liste par catégorie) ; avec recherche :
	// les 60 premiers résultats, groupés aussi.
	const hits = $derived(
		adding
			? searchOverridable(query, mode, isPart, settings, query.trim() ? 60 : 1000, filamentCount)
			: []
	);
	const groups = $derived(groupByCategory(hits));
	const clip = $derived(settingsClipboard.value);
	let confirmReset = $state(false);
	let feedback = $state<string | null>(null);

	function copy() {
		settingsClipboard.copy(title, settings);
		feedback = `${keys.length} réglage(s) copié(s).`;
	}
	function paste() {
		if (!clip) return;
		const r = pasteSettings(settings, clip.settings, isPart);
		onchange(r.settings);
		feedback =
			`${r.pasted} réglage(s) collé(s)` +
			(r.skipped.length ? `, ${r.skipped.length} ignoré(s) (non applicables ici).` : '.');
	}
	function resetAll() {
		confirmReset = false;
		onchange({});
		feedback = 'Surcharges retirées : le profil d’impression s’applique.';
	}

	function add(key: string) {
		onchange(addOverride(settings, key, base));
		query = '';
		adding = false;
	}

	function setValue(key: string, value: unknown) {
		onchange({ ...settings, [key]: value });
	}

	function focusOnMount(node: HTMLInputElement) {
		node.focus();
	}
</script>

<section class="flex flex-col gap-2 rounded border border-border p-2" data-testid={testid}>
	<div class="flex items-center justify-between gap-2">
		<span class="min-w-0 truncate text-sm font-medium text-content" {title}
			>{title}{#if keys.length}<span
					class="ws-badge ml-1.5 align-middle"
					data-testid="object-settings-count"
					data-tip="{keys.length} réglage(s) surchargé(s)">{keys.length}</span
				>{/if}</span
		>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
			data-testid="object-settings-add"
			onclick={() => (adding = !adding)}>{adding ? 'Fermer' : '+ Réglage'}</button
		>
	</div>
	<!-- Copier / coller / tout réinitialiser (Orca : menu de l'élément Réglages). -->
	<div class="flex flex-wrap items-center gap-1" role="group" aria-label="Actions sur les réglages">
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="object-settings-copy"
			disabled={keys.length === 0}
			data-tip="Copier ces réglages pour les coller sur un autre objet"
			onclick={copy}>Copier</button
		>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="object-settings-paste"
			disabled={!clip}
			data-tip={clip
				? `Coller les ${Object.keys(clip.settings).length} réglage(s) de « ${clip.source} »`
				: 'Rien à coller'}
			onclick={paste}>Coller</button
		>
		{#if confirmReset}
			<span class="flex items-center gap-1 text-xs text-content" role="alert">
				Retirer les {keys.length} surcharges ?
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-danger"
					data-testid="object-settings-reset-confirm"
					onclick={resetAll}>Oui</button
				>
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-ghost"
					onclick={() => (confirmReset = false)}>Non</button
				>
			</span>
		{:else}
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-ghost"
				data-testid="object-settings-reset"
				disabled={keys.length === 0}
				data-tip="Retirer toutes les surcharges de cet élément"
				onclick={() => (confirmReset = true)}>Tout réinitialiser</button
			>
		{/if}
	</div>
	{#if feedback}
		<p class="text-xs text-content-subtle" role="status">{feedback}</p>
	{/if}

	{#if adding}
		<input
			type="search"
			class="ws-field w-full"
			placeholder="Rechercher un réglage (clé ou libellé)"
			aria-label="Rechercher un réglage à surcharger"
			data-testid="object-settings-search"
			bind:value={query}
			use:focusOnMount
		/>
		<ul class="max-h-64 overflow-auto text-sm" data-testid="object-settings-results">
			{#each groups as group (group.category)}
				<li
					class="sticky top-0 bg-surface-raised px-1 pt-1.5 pb-0.5 text-2xs font-semibold tracking-wide text-content-subtle uppercase"
					role="presentation"
				>
					{group.category ? $t(group.category) : 'Autres'} · {group.defs.length}
				</li>
				{#each group.defs as def (def.key)}
					<li>
						<button
							type="button"
							class="flex w-full justify-between gap-2 rounded px-1 py-0.5 text-left hover:bg-surface-sunken"
							data-testid="object-settings-result"
							data-key={def.key}
							onclick={() => add(def.key)}
						>
							<span class="truncate">{$t(def.label)}</span>
							<span class="shrink-0 font-mono text-2xs text-content-subtle">{def.key}</span>
						</button>
					</li>
				{/each}
			{:else}
				<li class="px-1 text-xs text-content-subtle">Aucun réglage correspondant</li>
			{/each}
		</ul>
	{/if}

	{#if keys.length === 0 && !adding}
		<p class="text-xs text-content-subtle">
			Aucune surcharge : l'objet suit le profil d'impression.
		</p>
	{/if}

	<ul class="flex flex-col gap-1">
		{#each keys as key (key)}
			{@const def = PARAMS[key]}
			<li class="flex items-start gap-1" data-testid="object-setting" data-key={key}>
				<div class="min-w-0 flex-1">
					{#if def}
						<OptionLine {def} compact bind:value={() => settings[key], (v) => setValue(key, v)} />
					{:else}
						<span class="text-sm">{key} = {String(settings[key])}</span>
					{/if}
				</div>
				<button
					type="button"
					class="flex h-6 w-6 shrink-0 items-center justify-center rounded opacity-70 hover:bg-overlay hover:opacity-100"
					aria-label={`Retirer la surcharge ${key}`}
					data-testid="object-setting-remove"
					onclick={() => onchange(removeOverride(settings, key))}
				>
					<IconX class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			</li>
		{/each}
	</ul>
</section>
