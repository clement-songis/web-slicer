<script lang="ts">
	import { hasLiveControls, hostMeta } from './hosts';
	// Onglet « Appareil » (T117, US8 ; parité onglet Device d'Orca pour une
	// imprimante Klipper/Moonraker) : connexion, travail en cours (progression,
	// temps écoulé/restant, couche), températures et consignes, pause / reprise /
	// annulation, déplacements manuels et mise à l'origine, ventilateur, fichiers
	// de l'imprimante avec lancement, webcam. Les contrôles dont la route backend
	// n'existe pas encore (fichiers, G-code) sont détectés et désactivés avec une
	// explication, au lieu d'échouer au clic. Logique pure : `device.ts`,
	// `printers.ts`.
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import {
		listPrinters,
		getPrinterStatus,
		pausePrinter,
		resumePrinter,
		cancelPrinter,
		listPrinterFiles,
		startPrinterFile,
		sendPrinterGcode,
		type PrinterFile
	} from '$lib/api/printers';
	import {
		fromStatusResponse,
		stateMeta,
		progressPercent,
		canPause,
		canResume,
		canCancel,
		type PrinterStatusView
	} from './printers';
	import {
		fanGcode,
		formatClock,
		formatSize,
		homeGcode,
		isMissingRoute,
		loadWebcamUrl,
		moveGcode,
		remainingSeconds,
		saveWebcamUrl,
		temperatureGcode,
		type Axis,
		type ExtendedStatus,
		type Heater
	} from './device';
	import type { PrinterResponse } from '$lib/api/types';

	type Cap = 'unknown' | 'yes' | 'no';
	interface Live {
		view: PrinterStatusView;
		ext: ExtendedStatus;
		at: number;
	}

	let printers = $state<PrinterResponse[]>([]);
	let selectedId = $state<string | null>(null);
	let live = $state<Record<string, Live | undefined>>({});
	let offline = $state<Record<string, boolean>>({});
	let loading = $state(true);
	let error = $state<string | null>(null);
	let notice = $state<string | null>(null);
	let busy = $state(false);
	let now = $state(Date.now());

	// Capacités du backend (routes de relais Moonraker), sondées une fois.
	let capFiles = $state<Cap>('unknown');
	let capGcode = $state<Cap>('unknown');
	let files = $state<PrinterFile[]>([]);
	let filesError = $state<string | null>(null);

	const printer = $derived(printers.find((p) => p.id === selectedId) ?? null);
	const cur = $derived(selectedId ? live[selectedId] : undefined);
	const st = $derived(cur?.view);
	const meta = $derived(stateMeta(st?.state ?? ''));
	const elapsed = $derived(cur?.ext.print_duration ?? 0);
	const remaining = $derived(st ? remainingSeconds(st.progress, elapsed) : null);
	const active = $derived(st?.state === 'printing' || st?.state === 'paused');

	// Consignes saisies (préremplies à la cible courante).
	let targets = $state<Record<Heater, string>>({ extruder: '', bed: '' });
	let step = $state(10);
	let fan = $state(0);
	let webcamUrl = $state('');
	let webcamDraft = $state('');
	let webcamTick = $state(0);
	// Webcam : URL fournie par l'imprimante (statut, relayée depuis Moonraker),
	// l'URL saisie localement en est une surcharge. Chemin relatif → résolu sur
	// l'URL Moonraker.
	const printerWebcam = $derived.by(() => {
		const raw = (cur?.ext.webcam_url ?? '').trim();
		if (!raw) return '';
		try {
			return new URL(raw, printer?.moonraker_url || undefined).href;
		} catch {
			return raw;
		}
	});
	const webcamSrc = $derived(webcamUrl || printerWebcam);
	let confirmCancel = $state(false);

	async function refresh(id: string) {
		// Hôtes autres que Moonraker : pas d'état (409 côté serveur).
		const target = printers.find((p) => p.id === id);
		if (target && !hasLiveControls(target)) return;
		try {
			const s = await getPrinterStatus(id);
			live = {
				...live,
				[id]: { view: fromStatusResponse(s), ext: s as unknown as ExtendedStatus, at: Date.now() }
			};
			offline = { ...offline, [id]: false };
		} catch (e) {
			offline = { ...offline, [id]: true };
			if (e instanceof ApiError && id === selectedId) error = e.message;
		}
	}

	async function loadFiles(id: string) {
		try {
			files = (await listPrinterFiles(id)).sort((a, b) => b.modified - a.modified);
			capFiles = 'yes';
			filesError = null;
		} catch (e) {
			if (isMissingRoute(e)) capFiles = 'no';
			else filesError = e instanceof ApiError ? e.message : 'liste des fichiers indisponible';
		}
	}

	async function probeGcode(id: string) {
		try {
			await sendPrinterGcode(id, '');
			capGcode = 'yes';
		} catch (e) {
			capGcode = isMissingRoute(e) ? 'no' : 'yes';
		}
	}

	async function run(label: string, action: () => Promise<unknown>) {
		if (!selectedId) return;
		busy = true;
		error = null;
		try {
			await action();
			notice = label;
			await refresh(selectedId);
		} catch (e) {
			error = e instanceof ApiError ? e.message : `${label} : commande refusée`;
		} finally {
			busy = false;
		}
	}
	const gcode = (label: string, script: string) =>
		run(label, () => sendPrinterGcode(selectedId!, script));

	function select(id: string) {
		selectedId = id;
		error = null;
		notice = null;
		webcamUrl = loadWebcamUrl(id);
		webcamDraft = webcamUrl;
		const target = printers.find((p) => p.id === id);
		if (target && !hasLiveControls(target)) return;
		void refresh(id);
		void loadFiles(id);
		if (capGcode === 'unknown') void probeGcode(id);
	}

	// Consignes : suivre la cible de l'imprimante tant que le champ n'est pas édité.
	let editing = $state<Heater | null>(null);
	$effect(() => {
		if (!st) return;
		if (editing !== 'extruder')
			targets.extruder = st.extruderTarget ? String(Math.round(st.extruderTarget)) : '';
		if (editing !== 'bed') targets.bed = st.bedTarget ? String(Math.round(st.bedTarget)) : '';
		if (cur?.ext.fan_speed != null) fan = Math.round(cur.ext.fan_speed * 100);
	});

	onMount(() => {
		void (async () => {
			try {
				printers = await listPrinters();
				const first = printers.find((p) => hasLiveControls(p)) ?? printers[0];
				if (first) select(first.id);
				await Promise.all(printers.filter((p) => p.id !== first?.id).map((p) => refresh(p.id)));
			} catch (e) {
				error = e instanceof ApiError ? e.message : 'chargement des imprimantes impossible';
			} finally {
				loading = false;
			}
		})();
		// Suivi en direct (l'onglet Device d'Orca se rafraîchit en continu).
		const poll = setInterval(() => {
			if (selectedId && document.visibilityState === 'visible') void refresh(selectedId);
		}, 3000);
		const clock = setInterval(() => {
			now = Date.now();
			if (webcamSrc) webcamTick++;
		}, 1000);
		return () => {
			clearInterval(poll);
			clearInterval(clock);
		};
	});

	const PRESETS: { label: string; extruder: number; bed: number }[] = [
		{ label: 'PLA', extruder: 210, bed: 60 },
		{ label: 'PETG', extruder: 240, bed: 80 },
		{ label: 'ABS', extruder: 250, bed: 100 },
		{ label: 'Arrêt', extruder: 0, bed: 0 }
	];
	const missingTip = (route: string) =>
		`Nécessite la route backend ${route} (relais Moonraker), pas encore disponible`;
	const ago = (at: number) => Math.max(0, Math.round((now - at) / 1000));
</script>

{#snippet temp(heater: Heater, label: string, current: number, target: number)}
	<div class="flex items-center gap-2" data-testid="device-temp-{heater}">
		<span class="w-14 text-sm text-content">{label}</span>
		<span class="w-20 text-right text-lg font-medium whitespace-nowrap tabular-nums text-content"
			>{Math.round(current)} °C</span
		>
		<span class="w-16 text-xs whitespace-nowrap tabular-nums text-content-subtle"
			>{target > 0 ? `→ ${Math.round(target)} °C` : 'arrêt'}</span
		>
		<input
			type="number"
			min="0"
			max={heater === 'extruder' ? 350 : 150}
			class="ws-field ws-field-sm w-20 text-right"
			aria-label="Consigne {label} (°C)"
			placeholder="°C"
			bind:value={targets[heater]}
			onfocus={() => (editing = heater)}
			onblur={() => (editing = null)}
			onkeydown={(e) =>
				e.key === 'Enter' &&
				gcode(
					`Consigne ${label.toLowerCase()} envoyée`,
					temperatureGcode(heater, Number(targets[heater]) || 0)
				)}
			disabled={capGcode !== 'yes'}
		/>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary"
			disabled={capGcode !== 'yes' || busy}
			data-tip={capGcode === 'no' ? missingTip('POST /api/printers/{id}/gcode') : undefined}
			onclick={() =>
				gcode(
					`Consigne ${label.toLowerCase()} envoyée`,
					temperatureGcode(heater, Number(targets[heater]) || 0)
				)}>Régler</button
		>
	</div>
{/snippet}

{#snippet jog(axis: Axis, sign: number, label: string, cls: string)}
	<button
		type="button"
		class="ws-icon-btn border-border-strong {cls}"
		aria-label="{axis}{sign > 0 ? '+' : '−'}{step} mm"
		disabled={capGcode !== 'yes' || busy || active}
		onclick={() => gcode(`${axis} ${sign > 0 ? '+' : '−'}${step} mm`, moveGcode(axis, sign * step))}
		>{label}</button
	>
{/snippet}

<div class="mx-auto flex w-full max-w-6xl flex-col gap-4 p-6" data-testid="device-panel">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h2 class="ws-page-title">Appareil</h2>
		<a href={resolve('/printers')} class="ws-btn ws-btn-sm ws-btn-secondary"
			>Gérer les imprimantes</a
		>
	</div>

	{#if loading}
		<p class="text-sm text-content-subtle" role="status">Chargement…</p>
	{:else if printers.length === 0}
		<div
			class="rounded-lg border border-border bg-surface-raised p-8 text-center text-content-muted"
		>
			<p>Aucune imprimante déclarée.</p>
			<a href={resolve('/printers')} class="text-primary hover:underline">Déclarer une imprimante</a
			>
		</div>
	{:else}
		<!-- Sélecteur d'imprimante (liste d'Orca), pastille de connexion. -->
		{#if printers.length > 1}
			<div class="flex flex-wrap gap-1.5" role="tablist" aria-label="Imprimantes">
				{#each printers as p (p.id)}
					<button
						type="button"
						role="tab"
						aria-selected={p.id === selectedId}
						class="ws-chip {p.id === selectedId ? 'ws-chip-on' : ''}"
						onclick={() => select(p.id)}
					>
						<span
							class="h-2 w-2 rounded-full {offline[p.id]
								? 'bg-danger'
								: live[p.id]
									? 'bg-success'
									: 'bg-overlay'}"
							aria-hidden="true"
						></span>
						{p.name}
					</button>
				{/each}
			</div>
		{/if}

		{#if error}
			<div class="rounded-md bg-danger-soft px-3 py-2 text-sm text-danger-content" role="alert">
				{error}
			</div>
		{:else if notice}
			<div class="rounded-md bg-success-soft px-3 py-2 text-sm text-success-content" role="status">
				{notice}
			</div>
		{/if}

		{#if printer}
			<!-- En-tête : nom, URL, connexion, état. -->
			<section
				class="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-surface-raised px-4 py-3"
			>
				<div class="min-w-0 flex-1">
					<div class="font-medium text-content">{printer.name}</div>
					<div class="truncate text-xs text-content-subtle">
						{printer.moonraker_url
							? `${hostMeta(printer.host_type).label} · ${printer.moonraker_url}`
							: 'Connexion non déclarée'}
					</div>
				</div>
				{#if hasLiveControls(printer)}
					<span
						class="flex items-center gap-1.5 text-xs text-content-muted"
						data-testid="device-connection"
					>
						<span
							class="h-2 w-2 rounded-full {offline[printer.id]
								? 'bg-danger'
								: cur
									? 'bg-success'
									: 'bg-overlay'}"
							aria-hidden="true"
						></span>
						{#if offline[printer.id]}Hors ligne{:else if cur}Connectée · mise à jour il y a {ago(
								cur.at
							)} s{:else}Connexion…{/if}
					</span>
					<span class="rounded px-2 py-0.5 text-xs {meta.badge}" data-testid="device-state"
						>{st ? meta.label : '—'}</span
					>
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary"
						onclick={() => refresh(printer.id)}>Rafraîchir</button
					>
				{/if}
			</section>

			{#if !hasLiveControls(printer)}
				<p
					class="rounded-lg border border-dashed border-border px-4 py-6 text-center text-sm text-content-muted"
					data-testid="device-no-live"
				>
					{printer.moonraker_url
						? `${hostMeta(printer.host_type).label} : envoi des fichiers seulement. Le suivi en direct, la webcam et les commandes sont disponibles avec Moonraker.`
						: 'Aucune connexion déclarée : ajoutez-la depuis la page Imprimantes (« Connexion… »).'}
				</p>
			{:else}
				<div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
					<div class="flex min-w-0 flex-col gap-4">
						<!-- Webcam -->
						<section
							class="flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-4"
						>
							<h3 class="ws-section-title">Caméra</h3>
							{#if webcamSrc}
								<img
									src="{webcamSrc}{webcamSrc.includes('?') ? '&' : '?'}t={webcamTick}"
									alt="Webcam de {printer.name}"
									class="aspect-video w-full rounded-md bg-surface-sunken object-contain"
									data-testid="device-webcam"
								/>
							{:else}
								<div
									class="flex h-16 items-center justify-center rounded-md border border-dashed border-border text-sm text-content-subtle"
								>
									Aucune webcam déclarée par l'imprimante.
								</div>
							{/if}
							<p class="text-xs text-content-subtle" data-testid="device-webcam-source">
								{#if webcamUrl}
									URL locale (surcharge){printerWebcam
										? ` — l'imprimante déclare ${printerWebcam}`
										: ''}.
								{:else if printerWebcam}
									URL déclarée par l'imprimante (Moonraker).
								{:else}
									Moonraker ne déclare pas de webcam ; vous pouvez saisir une URL.
								{/if}
							</p>
							<form
								class="flex gap-2"
								onsubmit={(e) => {
									e.preventDefault();
									saveWebcamUrl(printer.id, webcamDraft);
									webcamUrl = webcamDraft.trim();
								}}
							>
								<input
									class="ws-field ws-field-sm min-w-0 flex-1"
									placeholder={printerWebcam ||
										"URL d'instantané ou de flux MJPEG (ex. http://imprimante/webcam/?action=snapshot)"}
									aria-label="URL de la webcam"
									bind:value={webcamDraft}
								/>
								<button type="submit" class="ws-btn ws-btn-sm ws-btn-secondary">Enregistrer</button>
								{#if webcamUrl}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-ghost"
										data-testid="device-webcam-reset"
										onclick={() => {
											saveWebcamUrl(printer.id, '');
											webcamUrl = '';
											webcamDraft = '';
										}}>{printerWebcam ? "URL de l'imprimante" : 'Effacer'}</button
									>
								{/if}
							</form>
						</section>

						<!-- Travail en cours -->
						<section
							class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
							data-testid="device-job"
						>
							<div class="flex items-center justify-between gap-2">
								<h3 class="ws-section-title">Impression</h3>
								<span class="text-2xl font-semibold tabular-nums text-content"
									>{st ? progressPercent(st.progress) : 0} %</span
								>
							</div>
							<p class="truncate text-sm text-content">
								{st?.filename || 'Aucun fichier en cours'}
							</p>
							<div
								class="h-2.5 w-full overflow-hidden rounded-full bg-overlay"
								role="progressbar"
								aria-valuemin="0"
								aria-valuemax="100"
								aria-valuenow={st ? progressPercent(st.progress) : 0}
								aria-label="Progression"
							>
								<div
									class="h-full rounded-full bg-primary transition-[width]"
									style:width="{st ? progressPercent(st.progress) : 0}%"
								></div>
							</div>
							<dl class="grid grid-cols-3 gap-3 text-sm">
								<div>
									<dt class="text-xs text-content-subtle">Écoulé</dt>
									<dd class="tabular-nums text-content">{active ? formatClock(elapsed) : '—'}</dd>
								</div>
								<div>
									<dt
										class="text-xs text-content-subtle"
										data-tip="Extrapolé depuis la progression et le temps écoulé"
									>
										Restant (estimé)
									</dt>
									<dd class="tabular-nums text-content" data-testid="device-remaining">
										{active ? formatClock(remaining) : '—'}
									</dd>
								</div>
								<div>
									<dt class="text-xs text-content-subtle">Couche</dt>
									<dd
										class="tabular-nums text-content"
										data-testid="device-layer"
										data-tip={cur?.ext.current_layer == null
											? 'Couche non fournie par le backend (print_stats.info)'
											: undefined}
									>
										{cur?.ext.current_layer != null
											? `${cur.ext.current_layer} / ${cur.ext.total_layer ?? '?'}`
											: '—'}
									</dd>
								</div>
							</dl>
							<div class="flex flex-wrap gap-2">
								<button
									type="button"
									class="ws-btn ws-btn-secondary"
									disabled={!st || !canPause(st.state) || busy}
									onclick={() => run('Impression en pause', () => pausePrinter(printer.id))}
									>Pause</button
								>
								<button
									type="button"
									class="ws-btn ws-btn-primary"
									disabled={!st || !canResume(st.state) || busy}
									onclick={() => run('Impression reprise', () => resumePrinter(printer.id))}
									>Reprendre</button
								>
								{#if confirmCancel}
									<span class="flex items-center gap-2 text-sm text-content" role="alert">
										Annuler l'impression ?
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-danger"
											onclick={() => {
												confirmCancel = false;
												void run('Impression annulée', () => cancelPrinter(printer.id));
											}}>Oui, annuler</button
										>
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											onclick={() => (confirmCancel = false)}>Non</button
										>
									</span>
								{:else}
									<button
										type="button"
										class="ws-btn ws-btn-secondary"
										disabled={!st || !canCancel(st.state) || busy}
										onclick={() => (confirmCancel = true)}>Annuler</button
									>
								{/if}
							</div>
						</section>

						<!-- Fichiers de l'imprimante -->
						<section
							class="flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-4"
							data-testid="device-files"
						>
							<div class="flex items-center justify-between">
								<h3 class="ws-section-title">Fichiers de l'imprimante</h3>
								{#if capFiles === 'yes'}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										onclick={() => loadFiles(printer.id)}>Actualiser</button
									>
								{/if}
							</div>
							{#if capFiles === 'no'}
								<p class="text-sm text-content-subtle">
									Liste indisponible : le backend ne relaie pas encore <code
										>/server/files/list</code
									>
									(route proposée : <code>GET /api/printers/{'{id}'}/files</code>). L'envoi depuis
									l'aperçu reste possible.
								</p>
							{:else if filesError}
								<p class="text-sm text-danger" role="alert">{filesError}</p>
							{:else if files.length === 0}
								<p class="text-sm text-content-subtle">
									{capFiles === 'unknown' ? 'Chargement…' : 'Aucun fichier.'}
								</p>
							{:else}
								<ul class="divide-y divide-border">
									{#each files as f (f.path)}
										<li class="flex items-center gap-3 py-1.5 text-sm">
											<span class="min-w-0 flex-1 truncate text-content" title={f.path}
												>{f.path}</span
											>
											<span class="text-xs tabular-nums text-content-subtle"
												>{formatSize(f.size)}</span
											>
											<span class="hidden text-xs tabular-nums text-content-subtle sm:inline"
												>{new Date(f.modified * 1000).toLocaleString('fr-FR', {
													dateStyle: 'short',
													timeStyle: 'short'
												})}</span
											>
											<button
												type="button"
												class="ws-btn ws-btn-sm ws-btn-secondary"
												disabled={active || busy}
												data-tip={active ? 'Une impression est en cours' : undefined}
												onclick={() =>
													run(`Impression de « ${f.path} » lancée`, () =>
														startPrinterFile(printer.id, f.path)
													)}>Imprimer</button
											>
										</li>
									{/each}
								</ul>
							{/if}
						</section>
					</div>

					<!-- Colonne de contrôle (températures, axes, ventilateur). -->
					<div class="flex flex-col gap-4">
						<section
							class="flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-4"
						>
							<h3 class="ws-section-title">Températures</h3>
							{@render temp('extruder', 'Buse', st?.extruderTemp ?? 0, st?.extruderTarget ?? 0)}
							{@render temp('bed', 'Plateau', st?.bedTemp ?? 0, st?.bedTarget ?? 0)}
							<div class="flex flex-wrap gap-1.5 pt-1">
								{#each PRESETS as p (p.label)}
									<button
										type="button"
										class="ws-chip"
										disabled={capGcode !== 'yes' || busy}
										data-tip={p.extruder
											? `Buse ${p.extruder} °C, plateau ${p.bed} °C`
											: 'Couper les chauffes'}
										onclick={() =>
											gcode(
												`Préchauffe ${p.label}`,
												`${temperatureGcode('extruder', p.extruder)}\n${temperatureGcode('bed', p.bed)}`
											)}>{p.label}</button
									>
								{/each}
							</div>
						</section>

						<section
							class="flex flex-col gap-3 rounded-lg border border-border bg-surface-raised p-4"
							data-testid="device-motion"
						>
							<div class="flex items-center justify-between">
								<h3 class="ws-section-title">Déplacements</h3>
								<div class="ws-seg" role="group" aria-label="Pas de déplacement">
									{#each [0.1, 1, 10, 50] as s (s)}
										<button
											type="button"
											class="ws-seg-item {step === s ? 'ws-seg-on' : ''}"
											aria-pressed={step === s}
											onclick={() => (step = s)}>{s}</button
										>
									{/each}
								</div>
							</div>
							<div class="flex items-center justify-center gap-6">
								<div class="grid grid-cols-3 gap-1" role="group" aria-label="Axes X et Y">
									<span></span>{@render jog('Y', 1, 'Y+', '')}<span></span>
									{@render jog('X', -1, 'X−', '')}
									<button
										type="button"
										class="ws-icon-btn border-border-strong"
										aria-label="Origine X Y"
										data-tip="Origine XY (G28 X Y)"
										disabled={capGcode !== 'yes' || busy || active}
										onclick={() => gcode('Origine XY', 'G28 X Y')}>⌂</button
									>
									{@render jog('X', 1, 'X+', '')}
									<span></span>{@render jog('Y', -1, 'Y−', '')}<span></span>
								</div>
								<div class="flex flex-col gap-1" role="group" aria-label="Axe Z">
									{@render jog('Z', 1, 'Z+', '')}
									<button
										type="button"
										class="ws-icon-btn border-border-strong"
										aria-label="Origine Z"
										disabled={capGcode !== 'yes' || busy || active}
										onclick={() => gcode('Origine Z', homeGcode('Z'))}>⌂Z</button
									>
									{@render jog('Z', -1, 'Z−', '')}
								</div>
							</div>
							<button
								type="button"
								class="ws-btn ws-btn-secondary"
								disabled={capGcode !== 'yes' || busy || active}
								onclick={() => gcode("Mise à l'origine", homeGcode())}
								>Origine de tous les axes</button
							>
						</section>

						<section
							class="flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-4"
						>
							<h3 class="ws-section-title">Ventilateur de pièce</h3>
							<div class="flex items-center gap-3">
								<input
									type="range"
									min="0"
									max="100"
									step="5"
									class="min-w-0 flex-1"
									aria-label="Ventilateur de pièce (%)"
									bind:value={fan}
									disabled={capGcode !== 'yes'}
									onchange={() => gcode(`Ventilateur à ${fan} %`, fanGcode(fan))}
								/>
								<span class="w-12 text-right text-sm tabular-nums text-content">{fan} %</span>
							</div>
						</section>

						{#if capGcode === 'no'}
							<p
								class="rounded-md border border-dashed border-border px-3 py-2 text-xs text-content-subtle"
								data-testid="device-gcode-missing"
							>
								Consignes, déplacements et ventilateur nécessitent la route backend
								<code>POST /api/printers/{'{id}'}/gcode</code> (relais
								<code>/printer/gcode/script</code>), pas encore disponible.
							</p>
						{/if}
					</div>
				</div>
			{/if}
		{/if}
	{/if}
</div>
