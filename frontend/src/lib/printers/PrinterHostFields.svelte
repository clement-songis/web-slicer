<script lang="ts">
	// Champs de connexion d'une imprimante (Orca : PhysicalPrinterDialog) : type
	// d'hôte, adresse, clé API ou utilisateur / mot de passe, imprimante du
	// serveur Repetier. Seuls les champs utiles au type choisi sont affichés.
	import { getHostPrinters } from '$lib/api/printers';
	import { ApiError } from '$lib/api/client';
	import { HOST_TYPES, hostMeta, type HostForm } from './hosts';

	interface Props {
		form: HostForm;
		/** Imprimante enregistrée (liste des imprimantes Repetier possible). */
		printerId?: string | null;
		/** Un secret est déjà enregistré (champ vide = conservé). */
		hasSecret?: boolean;
		hasPassword?: boolean;
		/** Avant de lister les imprimantes Repetier : enregistrer l'adresse/clé. */
		beforeListPrinters?: () => Promise<string | null>;
	}

	let {
		form = $bindable(),
		printerId = null,
		hasSecret = false,
		hasPassword = false,
		beforeListPrinters
	}: Props = $props();

	const meta = $derived(hostMeta(form.type));
	let slugs = $state<string[] | null>(null);
	let slugError = $state<string | null>(null);
	let loadingSlugs = $state(false);

	async function listSlugs() {
		loadingSlugs = true;
		slugError = null;
		try {
			const id = (await beforeListPrinters?.()) ?? printerId;
			if (!id) {
				slugError = "Enregistrez d'abord l'imprimante.";
				return;
			}
			slugs = await getHostPrinters(id);
			if (slugs.length && !form.slug) form.slug = slugs[0];
			if (!slugs.length) slugError = 'Aucune imprimante sur ce serveur.';
		} catch (e) {
			slugError = e instanceof ApiError ? e.message : 'Liste indisponible.';
		} finally {
			loadingSlugs = false;
		}
	}
</script>

<div class="flex flex-col gap-3" data-testid="host-fields">
	<label class="flex flex-col gap-1 text-sm">
		<span class="text-content-muted">Type d'hôte</span>
		<select
			class="ws-field"
			bind:value={form.type}
			data-testid="host-type"
			onchange={() => (slugs = null)}
		>
			{#each HOST_TYPES as h (h.id)}
				<option value={h.id}>{h.label}</option>
			{/each}
		</select>
	</label>
	<label class="flex flex-col gap-1 text-sm">
		<span class="text-content-muted">Adresse{form.type === 'moonraker' ? ' (facultatif)' : ''}</span
		>
		<input
			class="ws-field"
			bind:value={form.url}
			data-testid="printer-moonraker-url"
			placeholder={meta.placeholder}
			autocomplete="off"
		/>
	</label>
	{#if meta.authChoice}
		<div class="flex flex-col gap-1 text-sm" role="radiogroup" aria-label="Authentification">
			<span class="text-content-muted">Authentification</span>
			<div class="ws-seg self-start">
				{#each [['key', 'Clé API'], ['user', 'Utilisateur et mot de passe']] as [id, label] (id)}
					<button
						type="button"
						role="radio"
						aria-checked={form.auth === id}
						class="ws-seg-item {form.auth === id ? 'ws-seg-on' : ''}"
						data-testid={`host-auth-${id}`}
						onclick={() => (form.auth = id as 'key' | 'user')}>{label}</button
					>
				{/each}
			</div>
		</div>
	{/if}
	{#if meta.authChoice && form.auth === 'user'}
		<div class="grid grid-cols-2 gap-3">
			<label class="flex flex-col gap-1 text-sm">
				<span class="text-content-muted">Utilisateur</span>
				<input class="ws-field" bind:value={form.user} autocomplete="off" data-testid="host-user" />
			</label>
			<label class="flex flex-col gap-1 text-sm">
				<span class="text-content-muted">Mot de passe</span>
				<input
					class="ws-field"
					type="password"
					bind:value={form.password}
					autocomplete="new-password"
					placeholder={hasPassword ? 'Enregistré — laisser vide pour le garder' : ''}
					data-testid="host-password"
				/>
			</label>
		</div>
	{:else if meta.secret}
		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{meta.secret}</span>
			<input
				class="ws-field"
				type="password"
				bind:value={form.apiKey}
				autocomplete="new-password"
				placeholder={hasSecret ? 'Enregistrée — laisser vide pour la garder' : ''}
				data-testid="host-api-key"
			/>
		</label>
	{/if}
	{#if meta.printerSlug}
		<div class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">Imprimante sur le serveur</span>
			<div class="flex gap-2">
				{#if slugs?.length}
					<select class="ws-field flex-1" bind:value={form.slug} data-testid="host-slug">
						{#each slugs as s (s)}<option value={s}>{s}</option>{/each}
					</select>
				{:else}
					<input
						class="ws-field flex-1"
						bind:value={form.slug}
						placeholder="slug Repetier"
						data-testid="host-slug"
					/>
				{/if}
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					disabled={loadingSlugs || !form.url.trim()}
					data-testid="host-slug-list"
					onclick={listSlugs}>{loadingSlugs ? 'Recherche…' : 'Lister'}</button
				>
			</div>
			{#if slugError}<span class="text-xs text-danger-text" role="alert">{slugError}</span>{/if}
		</div>
	{/if}
	{#if form.type !== 'moonraker'}
		<p class="text-xs text-content-subtle" data-testid="host-limits">
			Envoi des fichiers et lancement de l'impression. Le suivi en direct et les commandes (pause,
			reprise, fichiers) restent réservés à Moonraker.
		</p>
	{/if}
</div>
