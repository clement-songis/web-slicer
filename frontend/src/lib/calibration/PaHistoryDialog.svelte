<script lang="ts">
	// Historique des calibrations Pressure Advance (Orca CaliHistoryDialog) :
	// résultats K par imprimante / buse / filament, stockés sur le serveur
	// (/api/calibrations/pa) ; ajouter, modifier, supprimer, appliquer au
	// filament actif.
	import { untrack } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { ApiError } from '$lib/api/client';
	import {
		createPa,
		deletePa,
		listPa,
		PA_METHODS,
		parseK,
		sortPa,
		takeLegacy,
		toInput,
		updatePa,
		type PaMethod,
		type PaRecord
	} from './pa-history';

	interface Props {
		open: boolean;
		printer: string;
		nozzle: string;
		filament: string;
		filamentId?: string | null;
		currentK: number | null;
		onclose: () => void;
		onapply: (k: number) => void;
	}
	let {
		open,
		printer,
		nozzle,
		filament,
		filamentId = null,
		currentK,
		onclose,
		onapply
	}: Props = $props();

	let rows = $state<PaRecord[]>([]);
	let loading = $state(false);
	// Filtres de la route (imprimante / buse / filament courants).
	let onlyPrinter = $state(true);
	let onlyNozzle = $state(true);
	let onlyFilament = $state(false);
	let kDraft = $state('');
	let method = $state<PaMethod>('pa_line');
	let note = $state('');
	let editing = $state<string | null>(null);
	let error = $state<string | null>(null);
	const nozzleMm = $derived(Number(nozzle) || 0.4);

	// Réponses périmées ignorées (filtres ou presets changés entre-temps).
	let seq = 0;
	async function refresh() {
		const mine = ++seq;
		loading = true;
		try {
			const list = await listPa({
				...(onlyPrinter ? { machine_preset: printer } : {}),
				...(onlyNozzle ? { nozzle_diameter: nozzleMm } : {}),
				...(onlyFilament ? { filament_preset: filament } : {})
			});
			if (mine === seq) rows = sortPa(list, filament);
		} catch (e) {
			if (mine === seq) error = e instanceof ApiError ? e.message : 'historique indisponible';
		} finally {
			if (mine === seq) loading = false;
		}
	}
	$effect(() => {
		if (!open) return;
		untrack(() => {
			kDraft = currentK !== null ? String(currentK) : '';
			note = '';
			editing = null;
			error = null;
			void (async () => {
				// Valeurs enregistrées dans le navigateur avant la route serveur.
				const legacy = takeLegacy(typeof localStorage === 'undefined' ? undefined : localStorage);
				for (const entry of legacy) await createPa(entry).catch(() => undefined);
				await refresh();
			})();
		});
	});
	$effect(() => {
		// Presets du projet chargés après l'ouverture (rechargement de page) :
		// la liste suit l'imprimante, la buse et le filament courants.
		void [onlyPrinter, onlyNozzle, onlyFilament, printer, nozzleMm, filament];
		if (open) untrack(() => void refresh());
	});

	async function add() {
		const k = parseK(kDraft);
		if (k === null) {
			error = 'K doit être un nombre entre 0 et 2 (ex. 0,04).';
			return;
		}
		error = null;
		try {
			await createPa({
				machine_preset: printer,
				filament_preset: filament,
				filament_id: filamentId,
				nozzle_diameter: nozzleMm,
				k_value: k,
				method,
				name: filament,
				note: note.trim() || null
			});
			note = '';
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'enregistrement impossible';
		}
	}
	async function update(row: PaRecord, raw: string) {
		const k = parseK(raw);
		if (k === null) {
			error = 'K invalide.';
			return;
		}
		error = null;
		editing = null;
		if (k === row.k_value) return;
		try {
			await updatePa(row.id, toInput(row, { k_value: k }));
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'modification impossible';
		}
	}
	async function remove(row: PaRecord) {
		try {
			await deletePa(row.id);
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'suppression impossible';
		}
	}
	const methodLabel = (m: string) => PA_METHODS.find((x) => x.id === m)?.label ?? m;
</script>

<Dialog
	{open}
	label="Historique Pressure Advance"
	title="Historique Pressure Advance"
	size="lg"
	{onclose}
	data-testid="pa-history-dialog"
>
	<div class="flex flex-col gap-3 text-sm">
		<p class="text-content-muted">
			{printer} · buse {nozzleMm} mm — valeurs K retenues après une calibration PA. Le filament actif
			est « {filament} ».
		</p>
		<form class="flex flex-wrap items-end gap-2" onsubmit={(e) => (e.preventDefault(), add())}>
			<label class="flex flex-col gap-1 text-xs text-content-muted"
				>K pour « {filament} »<input
					class="ws-field ws-field-sm w-24 tabular-nums"
					inputmode="decimal"
					bind:value={kDraft}
					data-testid="pa-k"
				/></label
			>
			<label class="flex flex-col gap-1 text-xs text-content-muted"
				>Méthode<select class="ws-field ws-field-sm" bind:value={method} data-testid="pa-method">
					{#each PA_METHODS as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
				</select></label
			>
			<label class="flex min-w-40 flex-1 flex-col gap-1 text-xs text-content-muted"
				>Note (facultative)<input
					class="ws-field ws-field-sm"
					bind:value={note}
					placeholder="ex. 0,002 par ligne"
				/></label
			>
			<button type="submit" class="ws-btn ws-btn-sm ws-btn-primary" data-testid="pa-add"
				>Enregistrer la mesure</button
			>
		</form>
		<fieldset class="flex flex-wrap items-center gap-4 text-xs text-content-muted">
			<legend class="sr-only">Filtres</legend>
			<label class="flex items-center gap-2">
				<input type="checkbox" bind:checked={onlyPrinter} data-testid="pa-only-printer" /> Cette imprimante
			</label>
			<label class="flex items-center gap-2">
				<input type="checkbox" bind:checked={onlyNozzle} data-testid="pa-only-nozzle" /> Buse {nozzleMm}
				mm
			</label>
			<label class="flex items-center gap-2">
				<input type="checkbox" bind:checked={onlyFilament} data-testid="pa-only-filament" /> Ce filament
			</label>
		</fieldset>
		{#if error}<p class="text-xs text-danger" role="alert">{error}</p>{/if}
		{#if loading && rows.length === 0}
			<p class="text-xs text-content-subtle" role="status">Chargement…</p>
		{:else if rows.length === 0}
			<p class="ws-empty rounded-lg border border-dashed border-border" data-testid="pa-empty">
				Aucune valeur pour ces filtres.
			</p>
		{:else}
			<table class="w-full text-sm" aria-busy={loading}>
				<thead class="text-left text-xs text-content-muted">
					<tr
						><th class="py-1">Filament</th><th class="py-1">Imprimante · buse</th><th class="py-1"
							>K</th
						><th class="py-1">Méthode</th><th class="py-1">Date</th><th></th></tr
					>
				</thead>
				<tbody>
					{#each rows as row (row.id)}
						<tr
							class="border-t border-border"
							data-testid="pa-row"
							data-filament={row.filament_preset}
						>
							<td
								class="py-1 {row.filament_preset === filament
									? 'font-medium text-content'
									: 'text-content-muted'}"
								>{row.filament_preset}{#if row.note}<span class="block text-xs text-content-subtle"
										>{row.note}</span
									>{/if}</td
							>
							<td class="py-1 text-xs text-content-muted" data-testid="pa-row-printer"
								>{row.machine_preset} · {row.nozzle_diameter} mm</td
							>
							<td class="py-1 tabular-nums">
								{#if editing === row.id}
									<input
										class="ws-field ws-field-sm w-24"
										value={String(row.k_value)}
										aria-label="Nouveau K"
										onkeydown={(e) => {
											if (e.key === 'Enter') void update(row, e.currentTarget.value);
											if (e.key === 'Escape') {
												e.stopPropagation();
												editing = null;
											}
										}}
										onblur={(e) => void update(row, e.currentTarget.value)}
									/>
								{:else}{row.k_value}{/if}
							</td>
							<td class="py-1 text-xs text-content-muted">{methodLabel(row.method)}</td>
							<td class="py-1 text-xs text-content-subtle"
								>{new Date(row.updated_at).toLocaleDateString()}</td
							>
							<td class="py-1 text-right whitespace-nowrap">
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-primary"
									data-testid="pa-apply"
									onclick={() => onapply(row.k_value)}>Appliquer au filament</button
								>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-ghost"
									onclick={() => (editing = row.id)}>Modifier</button
								>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-ghost text-danger"
									data-testid="pa-delete"
									onclick={() => void remove(row)}>Supprimer</button
								>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>Fermer</button>
	{/snippet}
</Dialog>
