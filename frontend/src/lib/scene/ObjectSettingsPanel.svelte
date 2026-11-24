<script lang="ts">
	// Réglages par objet / par volume (parité Orca « Add settings », panneau
	// des réglages d'objet) : liste des surcharges, ajout par recherche parmi les
	// clés autorisées (SettingsFactory::get_options), suppression. Purement
	// présentationnel : `onchange` reçoit le nouvel objet de surcharges.
	import { PARAMS } from '../../generated/params';
	import OptionLine from '#lib/settings/OptionLine.svelte';
	import type { DisplayMode } from '#lib/settings/filter';
	import { locale, t, tr } from '#lib/i18n';
	import { loadSettingsTranslations } from '#lib/settings/i18n';
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
	// Résultats affichés une fois ce dictionnaire chargé (phase 41d) : sous charge,
	// le chargement paresseux arrivait après l'ouverture et les catégories issues
	// du dictionnaire (« Layers and Perimeters »…) s'affichaient en anglais.
	let labelsReady = $state(false);
	$effect(() => {
		const loc = $locale;
		let live = true;
		labelsReady = false;
		loadSettingsTranslations(loc)
			.catch(() => {})
			.finally(() => {
				if (live) labelsReady = true;
			});
		return () => {
			live = false;
		};
	});

	let query = $state('');
	const keys = $derived(overriddenKeys(settings));
	// Sans recherche : tous les réglages ajoutables, groupés par catégorie
	// (« Ajouter des réglages » d'Orca, liste par catégorie) ; avec recherche :
	// les 60 premiers résultats, groupés aussi.
	// `$t` lu ici : la recherche compare aussi les libellés traduits (`tr`, non
	// réactif) ; le dictionnaire des réglages arrive en différé — sans cette
	// dépendance, une recherche saisie avant son chargement (« paroi ») restait
	// figée sur les seules traductions Paraglide et omettait « Ratio de débit de
	// la paroi extérieure ».
	const hits = $derived.by(() => {
		void $t;
		return adding
			? searchOverridable(query, mode, isPart, settings, query.trim() ? 60 : 1000, filamentCount)
			: [];
	});
	const groups = $derived(groupByCategory(hits));
	const clip = $derived(settingsClipboard.value);
	let confirmReset = $state(false);
	let feedback = $state<string | null>(null);

	function copy() {
		settingsClipboard.copy(title, settings);
		feedback = tr('{n} setting(s) copied.', { n: keys.length });
	}
	function paste() {
		if (!clip) return;
		const r = pasteSettings(settings, clip.settings, isPart);
		onchange(r.settings);
		feedback = r.skipped.length
			? tr('{n} setting(s) pasted, {m} skipped (not applicable here).', {
					n: r.pasted,
					m: r.skipped.length
				})
			: tr('{n} setting(s) pasted.', { n: r.pasted });
	}
	function resetAll() {
		confirmReset = false;
		onchange({});
		feedback = tr('Overrides removed: the process profile applies.');
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
		<span class="min-w-0 truncate text-sm font-medium text-content" {title}>
			{title}{#if keys.length}<span
					class="ws-badge ml-1.5 align-middle"
					data-testid="object-settings-count"
					data-tip={$t('{n} overridden setting(s)', { n: keys.length })}
				>
					{keys.length}
				</span>{/if}
		</span>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary shrink-0"
			data-testid="object-settings-add"
			onclick={() => (adding = !adding)}
		>
			{adding ? $t('Close') : $t('+ Setting')}
		</button>
	</div>
	<!-- Copier / coller / tout réinitialiser (Orca : menu de l'élément Réglages). -->
	<div class="flex flex-wrap items-center gap-1" role="group" aria-label={$t('Setting actions')}>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="object-settings-copy"
			disabled={keys.length === 0}
			data-tip={$t('Copy these settings to paste them onto another object')}
			onclick={copy}
		>
			{$t('Copy')}
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			data-testid="object-settings-paste"
			disabled={!clip}
			data-tip={clip
				? $t('Paste the {n} setting(s) from “{source}”', {
						n: Object.keys(clip.settings).length,
						source: clip.source
					})
				: $t('Nothing to paste')}
			onclick={paste}
		>
			{$t('Paste')}
		</button>
		{#if confirmReset}
			<span class="flex items-center gap-1 text-xs text-content" role="alert">
				{$t('Remove the {n} overrides?', { n: keys.length })}
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-danger"
					data-testid="object-settings-reset-confirm"
					onclick={resetAll}
				>
					{$t('Yes')}
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-ghost"
					onclick={() => (confirmReset = false)}
				>
					{$t('No')}
				</button>
			</span>
		{:else}
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-ghost"
				data-testid="object-settings-reset"
				disabled={keys.length === 0}
				data-tip={$t('Remove all overrides from this item')}
				onclick={() => (confirmReset = true)}
			>
				{$t('Reset all')}
			</button>
		{/if}
	</div>
	{#if feedback}
		<p class="text-xs text-content-subtle" role="status">{feedback}</p>
	{/if}

	{#if adding}
		<input
			type="search"
			class="ws-field w-full"
			placeholder={$t('Search a setting (key or label)')}
			aria-label={$t('Search a setting to override')}
			data-testid="object-settings-search"
			bind:value={query}
			use:focusOnMount
		/>
		<ul
			class="max-h-64 overflow-auto text-sm"
			data-testid="object-settings-results"
			aria-busy={labelsReady ? undefined : 'true'}
		>
			{#each labelsReady ? groups : [] as group (group.category)}
				<li
					class="sticky top-0 bg-surface-raised px-1 pt-1.5 pb-0.5 text-2xs font-semibold tracking-wide text-content-subtle uppercase"
					role="presentation"
				>
					{group.category ? $t(group.category) : $t('Others||category')} · {group.defs.length}
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
				<li class="px-1 text-xs text-content-subtle">
					{labelsReady ? $t('No matching setting') : $t('Loading settings…')}
				</li>
			{/each}
		</ul>
	{/if}

	{#if keys.length === 0 && !adding}
		<p class="text-xs text-content-subtle">
			{$t('No overrides: the object follows the process profile.')}
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
					aria-label={$t('Remove override {key}', { key })}
					data-testid="object-setting-remove"
					onclick={() => onchange(removeOverride(settings, key))}
				>
					<IconX class="h-3.5 w-3.5" aria-hidden="true" />
				</button>
			</li>
		{/each}
	</ul>
</section>
