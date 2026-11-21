<script lang="ts">
	// Onglet Appareil, hôtes sans suivi en direct ici (phase 39) : interface web
	// de l'hôte intégrée comme l'onglet Device d'Orca (MainFrame::load_printer_url,
	// PrintHost::get_print_host_webui : print_host_webui, sinon print_host). Le
	// navigateur peut refuser l'intégration (en-têtes de l'hôte, contenu mixte) :
	// lien externe toujours visible, et repli automatique sur ce lien.
	import { getResolvedPreset } from '$lib/api/presets';
	import type { PrinterResponse } from '$lib/api/types';
	import { hostMeta } from './hosts';
	import {
		EMBED_TIMEOUT_MS,
		blockedMessage,
		embedBlockedReason,
		loadWebuiPrefs,
		presetWebui,
		saveWebuiPrefs,
		webuiUrl,
		type WebuiPrefs
	} from './webui';

	interface Props {
		printer: PrinterResponse;
	}
	let { printer }: Props = $props();

	let prefs = $state<WebuiPrefs>({});
	let fromPreset = $state('');
	let editing = $state(false);
	let draft = $state('');
	let loaded = $state(false);
	let timedOut = $state(false);
	let reloadKey = $state(0);

	// Imprimante choisie : réglages locaux et print_host_webui du profil machine.
	$effect(() => {
		const id = printer.id;
		const presetId = printer.machine_preset_id;
		prefs = loadWebuiPrefs(id);
		fromPreset = '';
		editing = false;
		let alive = true;
		if (presetId)
			getResolvedPreset(presetId)
				.then((r) => {
					if (alive) fromPreset = presetWebui(r.values as Record<string, unknown>);
				})
				.catch(() => {});
		return () => {
			alive = false;
		};
	});

	const url = $derived(
		webuiUrl({ override: prefs.url, presetWebui: fromPreset, host: printer.moonraker_url })
	);
	const known = $derived(
		url
			? embedBlockedReason(url, typeof location === 'undefined' ? 'http:' : location.protocol)
			: null
	);
	const reason = $derived(prefs.noEmbed ? 'manual' : (known ?? (timedOut ? 'refused' : null)));
	const label = $derived(hostMeta(printer.host_type).label);

	// Iframe sans « load » au bout du délai : jugée refusée (repli sur le lien).
	const canEmbed = $derived(!!url && !prefs.noEmbed && known === null);
	$effect(() => {
		const go = canEmbed;
		void url;
		void reloadKey;
		loaded = false;
		timedOut = false;
		if (!go) return;
		const t = setTimeout(() => {
			if (!loaded) timedOut = true;
		}, EMBED_TIMEOUT_MS);
		return () => clearTimeout(t);
	});

	function update(next: WebuiPrefs) {
		prefs = next;
		saveWebuiPrefs(printer.id, next);
	}
	function saveDraft() {
		update({ ...prefs, url: draft.trim() || undefined });
		editing = false;
	}
</script>

<section
	class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
	data-testid="device-webui"
	data-webui-url={url || undefined}
	data-webui-state={!url ? 'none' : reason ? `link:${reason}` : loaded ? 'embedded' : 'loading'}
>
	<div class="flex flex-wrap items-center gap-2">
		<div class="min-w-0 flex-1">
			<h3 class="text-sm font-medium text-content">Interface de l’appareil</h3>
			<p class="truncate text-xs text-content-subtle">
				{url ? `${label} · ${url}` : `${label} : aucune adresse d’interface`}
			</p>
		</div>
		{#if url}
			<!-- Lien externe vers l'hôte (hors de l'application). -->
			<!-- eslint-disable svelte/no-navigation-without-resolve -->
			<a
				href={url}
				target="_blank"
				rel="noopener noreferrer"
				class="ws-btn ws-btn-sm ws-btn-secondary"
				data-testid="device-webui-open">Ouvrir dans un nouvel onglet</a
			>
			<!-- eslint-enable svelte/no-navigation-without-resolve -->
			{#if !reason}
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-secondary"
					onclick={() => (reloadKey += 1)}>Recharger</button
				>
			{/if}
			{#if known === null}
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-ghost"
					data-testid="device-webui-embed-toggle"
					onclick={() => update({ ...prefs, noEmbed: !prefs.noEmbed || undefined })}
					>{prefs.noEmbed ? 'Intégrer ici' : 'Lien seulement'}</button
				>
			{/if}
		{/if}
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-ghost"
			aria-expanded={editing}
			data-testid="device-webui-edit"
			onclick={() => {
				draft = prefs.url ?? '';
				editing = !editing;
			}}>Adresse…</button
		>
	</div>

	{#if editing}
		<form
			class="flex flex-wrap items-center gap-2"
			onsubmit={(e) => {
				e.preventDefault();
				saveDraft();
			}}
		>
			<input
				class="ws-field ws-field-sm min-w-0 flex-1"
				bind:value={draft}
				aria-label="Adresse de l’interface web"
				data-testid="device-webui-url"
				placeholder={fromPreset
					? `depuis le profil machine : ${fromPreset}`
					: printer.moonraker_url || 'http://imprimante.local'}
			/>
			<button type="submit" class="ws-btn ws-btn-sm ws-btn-primary">Enregistrer</button>
			<span class="w-full text-xs text-content-subtle"
				>Vide : adresse du profil machine (« Interface utilisateur de l’appareil »), sinon celle de
				la connexion. Mémorisée dans ce navigateur.</span
			>
		</form>
	{/if}

	{#if !url}
		<p class="text-sm text-content-muted">
			Renseignez l’adresse de l’imprimante (page Imprimantes, « Connexion… ») ou une adresse
			d’interface ci-dessus.
		</p>
	{:else if reason}
		<div
			class="flex flex-col items-center gap-3 rounded-md border border-dashed border-border px-4 py-8 text-center"
			data-testid="device-webui-fallback"
		>
			<p class="text-sm text-content-muted">{blockedMessage(reason)}</p>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={url} target="_blank" rel="noopener noreferrer" class="ws-btn ws-btn-primary"
				>Ouvrir l’interface de {label}</a
			>
			{#if reason === 'refused'}
				<button type="button" class="ws-btn ws-btn-sm ws-btn-ghost" onclick={() => (reloadKey += 1)}
					>Réessayer l’intégration</button
				>
			{/if}
		</div>
	{:else}
		{#key reloadKey}
			<iframe
				src={url}
				title="Interface web de {printer.name}"
				class="h-[70vh] min-h-96 w-full rounded-md border border-border bg-surface"
				sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-downloads allow-modals"
				referrerpolicy="no-referrer"
				data-testid="device-webui-frame"
				onload={() => (loaded = true)}
			></iframe>
		{/key}
		<p class="text-xs text-content-subtle">
			Page vide ou refusée ? L’hôte interdit peut-être l’intégration : utilisez « Ouvrir dans un
			nouvel onglet ».
		</p>
	{/if}
	<p class="text-xs text-content-subtle">
		L’envoi des fichiers se fait depuis l’éditeur ; le suivi en direct et les commandes intégrés
		sont réservés à Moonraker.
	</p>
</section>
