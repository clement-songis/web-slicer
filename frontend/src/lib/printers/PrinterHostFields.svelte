<script lang="ts">
	// Champs de connexion d'une imprimante (Orca : PhysicalPrinterDialog) : type
	// d'hôte, adresse, clé API ou utilisateur / mot de passe, imprimante du
	// serveur Repetier. Seuls les champs utiles au type choisi sont affichés.
	import { discoverPrinters, getHostPrinters } from '#lib/api/printers';
	import type { BonjourReply } from '#lib/api/types';
	import {
		ELEGOO_PROTOCOL_LABELS,
		discoveryHints,
		discoveryService,
		discoveryTimeoutMs,
		elegooProtocol,
		hostPrinterValue,
		profileHost,
		profilePlaceholder,
		type ProfileHost
	} from './hosts';
	import { getResolvedPreset } from '#lib/api/presets';
	import { ApiError } from '#lib/api/client';
	import { HOST_GROUPS, hostCapabilities, hostMeta, type HostForm } from './hosts';
	import { t, tr } from '#lib/i18n';
	import IconCheck from '~icons/lucide/check';
	import IconMinus from '~icons/lucide/minus';

	interface Props {
		form: HostForm;
		/** Imprimante enregistrée (liste des imprimantes Repetier possible). */
		printerId?: string | null;
		/** Un secret est déjà enregistré (champ vide = conservé). */
		hasSecret?: boolean;
		hasPassword?: boolean;
		/** Avant de lister les imprimantes Repetier : enregistrer l'adresse/clé. */
		beforeListPrinters?: () => Promise<string | null>;
		/** Preset machine de l'imprimante : ses réglages d'hôte (port, utilisateur,
		 *  n° de série, modèle) servent quand le champ reste vide. */
		machinePresetId?: string | null;
		/** Réglages d'hôte du profil machine (liable, pour la validation). */
		profile?: ProfileHost;
	}

	let {
		form = $bindable(),
		printerId = null,
		hasSecret = false,
		hasPassword = false,
		beforeListPrinters,
		machinePresetId = null,
		profile = $bindable({})
	}: Props = $props();

	// Lecture du profil machine (valeurs résolues de la chaîne d'héritage).
	$effect(() => {
		const id = machinePresetId;
		if (!id) {
			profile = {};
			return;
		}
		let alive = true;
		getResolvedPreset(id)
			.then((r) => {
				if (alive) profile = profileHost(r.values as Record<string, unknown>);
			})
			.catch(() => {
				if (alive) profile = {};
			});
		return () => {
			alive = false;
		};
	});
	const elegoo = $derived(form.type === 'elegoolink' ? elegooProtocol(profile.model) : null);

	const meta = $derived(hostMeta(form.type));
	const capabilities = $derived.by(() => {
		void $t; // libellés traduits : recalcul au changement de langue
		return hostCapabilities(form.type, profile.model);
	});
	/** Placeholder « depuis le profil machine : … » (réactif à la langue). */
	function fromProfile(value: string | undefined | null, fallback = ''): string {
		void $t;
		return profilePlaceholder(value, fallback);
	}
	let slugs = $state<string[] | null>(null);
	let slugError = $state<string | null>(null);
	let loadingSlugs = $state(false);

	// « Parcourir » (Orca BonjourDialog) : recherche DNS-SD depuis le serveur.
	const service = $derived(discoveryService(form.type));
	const hints = $derived.by(() => {
		void $t;
		return discoveryHints(service ?? '');
	});
	let found = $state<BonjourReply[] | null>(null);
	let browsing = $state(false);
	let browseError = $state<string | null>(null);
	$effect(() => {
		void form.type;
		found = null;
		browseError = null;
	});
	async function browse() {
		if (!service) return;
		browsing = true;
		browseError = null;
		found = null;
		try {
			found = await discoverPrinters(service, discoveryTimeoutMs(service));
		} catch (e) {
			browseError =
				e instanceof ApiError && (e.status === 404 || e.status === 405)
					? tr('Network search unavailable: the web-slicer server must be updated.')
					: e instanceof ApiError
						? e.message
						: tr('Search failed.');
		} finally {
			browsing = false;
		}
	}

	// Orca (Flashforge) : l'adresse et le n° de série de l'imprimante choisie.
	function useHost(host: BonjourReply) {
		form.url = host.full_address;
		const serial = host.txt?.serial_number;
		if (form.type === 'flashforge' && serial) form.serial = serial;
	}

	async function listSlugs() {
		loadingSlugs = true;
		slugError = null;
		try {
			const id = (await beforeListPrinters?.()) ?? printerId;
			if (!id) {
				slugError = tr('Save the printer first.');
				return;
			}
			slugs = await getHostPrinters(id);
			if (slugs.length && !form.slug) form.slug = hostPrinterValue(form.type, slugs[0]);
			if (!slugs.length) slugError = tr('No printer on this server.');
		} catch (e) {
			slugError = e instanceof ApiError ? e.message : tr('List unavailable.');
		} finally {
			loadingSlugs = false;
		}
	}
</script>

<div class="flex flex-col gap-3" data-testid="host-fields">
	<label class="flex flex-col gap-1 text-sm">
		<span class="text-content-muted">{$t('Host Type')}</span>
		<select
			class="ws-field"
			bind:value={form.type}
			data-testid="host-type"
			onchange={() => (slugs = null)}
		>
			{#each HOST_GROUPS as g (g.label)}
				<optgroup label={$t(g.label)}>
					{#each g.hosts as h (h.id)}
						<option value={h.id}>{h.label}</option>
					{/each}
				</optgroup>
			{/each}
		</select>
	</label>
	<!-- Fiche du type choisi : ce que l'hôte permet, d'un coup d'œil (Orca ne le dit
	     qu'à l'usage, par des boutons grisés). -->
	<ul class="flex flex-wrap gap-x-3 gap-y-1 text-xs" data-testid="host-capabilities">
		{#each capabilities as c (c.id)}
			<li
				class="inline-flex items-center gap-1 {c.ok
					? 'text-content-muted'
					: 'text-content-subtle line-through decoration-content-subtle/60'}"
				data-capability={c.id}
				data-ok={c.ok}
			>
				{#if c.ok}<IconCheck
						class="text-success-text h-3.5 w-3.5"
						aria-hidden="true"
					/>{:else}<IconMinus class="h-3.5 w-3.5" aria-hidden="true" />{/if}
				<span><span class="sr-only">{c.ok ? $t('Yes: ') : $t('No: ')}</span>{c.label}</span>
			</li>
		{/each}
	</ul>
	<label class="flex flex-col gap-1 text-sm">
		<span class="text-content-muted">
			{form.type === 'moonraker' ? $t('Address (optional)') : $t('Address')}
		</span>
		<div class="flex gap-2">
			<input
				class="ws-field min-w-0 flex-1"
				bind:value={form.url}
				data-testid="printer-moonraker-url"
				placeholder={meta.placeholder}
				autocomplete="off"
			/>
			{#if service}
				<button
					type="button"
					class="ws-btn ws-btn-secondary shrink-0"
					data-testid="host-browse"
					disabled={browsing}
					aria-busy={browsing}
					onclick={browse}
				>
					{browsing ? $t('Searching…') : $t('Browse network')}
				</button>
			{/if}
		</div>
	</label>
	{#if service && (browsing || found || browseError)}
		<div
			class="-mt-1 rounded-md border border-border px-3 py-2 text-sm"
			data-testid="host-browse-results"
			aria-live="polite"
		>
			{#if browsing}
				<p class="text-content-muted">{hints.waiting}</p>
			{:else if browseError}
				<p class="text-danger" role="alert">{browseError}</p>
			{:else if found && found.length === 0}
				<p class="text-content-muted">{hints.none}</p>
			{:else if found}
				<ul class="flex flex-col divide-y divide-border">
					{#each found as host (host.full_address + host.service_name)}
						<li class="flex items-center justify-between gap-3 py-1.5">
							<span class="min-w-0">
								<span class="block truncate text-content">
									{host.service_name || host.hostname}
								</span>
								<span class="block truncate text-xs text-content-subtle">
									{host.full_address}{host.hostname ? ` · ${host.hostname}` : ''}
								</span>
							</span>
							<button
								type="button"
								class="ws-btn ws-btn-sm {form.url === host.full_address
									? 'ws-btn-ghost'
									: 'ws-btn-primary'} shrink-0"
								data-testid="host-browse-use"
								disabled={form.url === host.full_address}
								onclick={() => useHost(host)}
							>
								{form.url === host.full_address ? $t('Chosen') : $t('Use')}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
	{#if meta.authChoice}
		<div class="flex flex-col gap-1 text-sm" role="radiogroup" aria-label={$t('Authentication')}>
			<span class="text-content-muted">{$t('Authentication')}</span>
			<div class="ws-seg self-start">
				{#each [['key', 'API key'], ['user', 'User and password']] as [id, label] (id)}
					<button
						type="button"
						role="radio"
						aria-checked={form.auth === id}
						class="ws-seg-item {form.auth === id ? 'ws-seg-on' : ''}"
						data-testid={`host-auth-${id}`}
						onclick={() => (form.auth = id as 'key' | 'user')}
					>
						{$t(label)}
					</button>
				{/each}
			</div>
		</div>
	{/if}
	{#if meta.authChoice && profile.auth}
		<p class="-mt-2 text-xs text-content-subtle" data-testid="host-auth-profile">
			{$t('Machine profile: {auth}', {
				auth: profile.auth === 'user' ? $t('user and password') : $t('API key||lower')
			})}{profile.auth !== form.auth ? $t(' — your choice above takes precedence') : ''}.
		</p>
	{/if}
	{#if elegoo}
		<p class="-mt-1 text-xs text-content-muted" data-testid="host-elegoo-protocol">
			{profile.model ? $t('Machine profile model: {model} — ', { model: profile.model }) : ''}{$t(
				ELEGOO_PROTOCOL_LABELS[elegoo]
			)}.
		</p>
	{/if}
	{#if meta.authChoice && form.auth === 'user'}
		<div class="grid grid-cols-2 gap-3">
			<label class="flex flex-col gap-1 text-sm">
				<span class="text-content-muted">{$t('User')}</span>
				<input
					class="ws-field"
					bind:value={form.user}
					autocomplete="off"
					placeholder={fromProfile(profile.user)}
					data-testid="host-user"
				/>
			</label>
			<label class="flex flex-col gap-1 text-sm">
				<span class="text-content-muted">{$t('Password')}</span>
				<input
					class="ws-field"
					type="password"
					bind:value={form.password}
					autocomplete="new-password"
					placeholder={hasPassword
						? $t('•••••• Saved')
						: profile.hasPassword
							? $t('from the machine profile')
							: ''}
					title={hasPassword ? $t('Leave empty to keep the saved password') : undefined}
					data-testid="host-password"
				/>
			</label>
		</div>
	{:else if meta.secret}
		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t(meta.secret)}</span>
			<input
				class="ws-field"
				type="password"
				bind:value={form.apiKey}
				autocomplete="new-password"
				placeholder={hasSecret ? $t('Saved — leave empty to keep it||key') : ''}
				data-testid="host-api-key"
			/>
		</label>
	{/if}
	{#if meta.serialNumber}
		<label class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t('Serial Number')}</span>
			<input
				class="ws-field"
				bind:value={form.serial}
				autocomplete="off"
				spellcheck="false"
				placeholder={profile.serial
					? fromProfile(profile.serial)
					: printerId
						? $t('Saved — leave empty to keep it||serial')
						: $t('e.g. SNMOMC9900728')}
				data-testid="host-serial"
			/>
		</label>
	{/if}
	{#if meta.printerSlug}
		<div class="flex flex-col gap-1 text-sm">
			<span class="text-content-muted">{$t(meta.slugLabel ?? 'Printer on the server')}</span>
			<div class="flex gap-2">
				{#if slugs?.length}
					<select class="ws-field flex-1" bind:value={form.slug} data-testid="host-slug">
						{#each slugs as s (s)}<option value={hostPrinterValue(form.type, s)}>{s}</option>{/each}
					</select>
				{:else}
					<input
						class="ws-field flex-1"
						bind:value={form.slug}
						placeholder={fromProfile(
							profile.port,
							form.type === 'obico' ? $t('Obico ID') : $t('Repetier slug')
						)}
						data-testid="host-slug"
					/>
				{/if}
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					disabled={loadingSlugs || !form.url.trim()}
					data-testid="host-slug-list"
					onclick={listSlugs}
				>
					{loadingSlugs ? $t('Searching…') : $t('List')}
				</button>
			</div>
			{#if slugError}<span class="text-danger-text text-xs" role="alert">{slugError}</span>{/if}
		</div>
	{/if}
	<p class="text-xs text-content-subtle" data-testid="host-help">{$t(meta.help)}</p>
	{#if form.type !== 'moonraker'}
		<p class="sr-only" data-testid="host-limits">
			{$t('Live monitoring and commands (pause, resume, files) remain reserved for Moonraker.')}
		</p>
	{/if}
</div>
