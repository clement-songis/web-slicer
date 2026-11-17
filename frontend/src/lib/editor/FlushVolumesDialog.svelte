<script lang="ts">
	// Volumes de purge du projet (Orca : « Flushing volumes », config projet) :
	// matrice départ → arrivée par paire de filaments, en-têtes aux couleurs des
	// slots, multiplicateur global. Brouillon local appliqué par « Appliquer ».
	import Dialog from '$lib/ui/Dialog.svelte';
	import { swatchStyle } from '$lib/ui/color';
	import { DEFAULT_FLUSH, resizeFlushMatrix } from './multi-filament';

	interface Props {
		open: boolean;
		/** Couleur CSS de chaque slot (nombre de slots = taille de la matrice). */
		colours: string[];
		matrix?: number[];
		multiplier?: number;
		onclose: () => void;
		onapply: (matrix: number[], multiplier: number) => void;
		/** Calcul moteur depuis les couleurs (null = jamais disponible). */
		compute?: ((colours: string[], multiplier: number) => Promise<number[]>) | null;
		/** Sonde de disponibilité du calcul moteur. */
		computeAvailable?: (() => Promise<boolean>) | null;
	}

	let {
		open,
		colours,
		matrix,
		multiplier = 1,
		onclose,
		onapply,
		compute = null,
		computeAvailable = null
	}: Props = $props();

	let canCompute = $state<boolean | null>(null);
	let computing = $state(false);
	let computeError = $state<string | null>(null);
	$effect(() => {
		if (!open || !computeAvailable) return;
		canCompute = null;
		computeError = null;
		void computeAvailable().then((ok) => (canCompute = ok));
	});
	async function runCompute() {
		if (!compute || computing) return;
		computing = true;
		computeError = null;
		try {
			const m = await compute(colours, 1);
			if (m.length === colours.length * colours.length) draft = m.map((v) => Math.round(v));
			else computeError = 'Réponse du moteur inattendue (taille de matrice).';
		} catch (e) {
			computeError = e instanceof Error ? e.message : 'Calcul impossible.';
		} finally {
			computing = false;
		}
	}

	const n = $derived(colours.length);
	let draft = $state<number[]>([]);
	let factor = $state(1);
	// Brouillon réinitialisé à chaque ouverture.
	$effect(() => {
		if (open) {
			draft = resizeFlushMatrix(matrix, colours.length);
			factor = multiplier;
		}
	});

	const at = (r: number, c: number) => draft[r * n + c] ?? 0;
	function setCell(r: number, c: number, raw: string) {
		const v = Number(raw);
		if (!Number.isFinite(v) || v < 0) return;
		const next = [...draft];
		next[r * n + c] = Math.round(v);
		draft = next;
	}
	const maxVolume = $derived(Math.max(0, ...draft) * factor);
</script>

<Dialog
	{open}
	label="Volumes de purge"
	title="Volumes de purge"
	size="lg"
	{onclose}
	data-testid="flush-dialog"
>
	<div class="flex flex-col gap-4">
		<p class="text-sm text-content-muted">
			Volume extrudé dans la tour d'essuyage (mm³) à chaque changement de filament, de la ligne
			(filament retiré) vers la colonne (filament chargé). Un passage du clair au foncé demande
			moins de purge que l'inverse.
		</p>
		<div class="overflow-x-auto">
			<table class="border-separate border-spacing-1 text-xs" data-testid="flush-matrix">
				<thead>
					<tr>
						<th scope="col" class="text-left font-normal text-content-subtle">De ↓ / Vers →</th>
						{#each colours as colour, c (c)}
							<th scope="col">
								<span
									class="inline-flex h-6 w-14 items-center justify-center rounded font-semibold ring-1 ring-border-strong"
									style={swatchStyle(colour)}>{c + 1}</span
								>
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each colours as colour, r (r)}
						<tr>
							<th scope="row">
								<span
									class="inline-flex h-6 w-10 items-center justify-center rounded font-semibold ring-1 ring-border-strong"
									style={swatchStyle(colour)}>{r + 1}</span
								>
							</th>
							{#each colours.keys() as c (c)}
								<td>
									{#if r === c}
										<span class="block w-14 text-center text-content-subtle" aria-hidden="true"
											>—</span
										>
									{:else}
										<input
											type="number"
											min="0"
											step="10"
											class="ws-field ws-field-sm w-14 text-center tabular-nums"
											aria-label={`Purge du filament ${r + 1} vers le filament ${c + 1} (mm³)`}
											data-testid={`flush-cell-${r + 1}-${c + 1}`}
											value={at(r, c)}
											onchange={(e) => setCell(r, c, e.currentTarget.value)}
										/>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<div class="flex flex-wrap items-center gap-3 text-sm">
			<label class="flex items-center gap-2">
				<span class="text-content-muted">Multiplicateur</span>
				<input
					type="number"
					min="0.1"
					step="0.1"
					class="ws-field ws-field-sm w-20 tabular-nums"
					data-testid="flush-multiplier"
					bind:value={factor}
				/>
			</label>
			<span class="text-xs text-content-subtle" data-testid="flush-max"
				>Purge maximale : {Math.round(maxVolume)} mm³</span
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary ml-auto"
				data-testid="flush-compute"
				data-available={canCompute === null ? 'unknown' : String(canCompute)}
				disabled={!compute || canCompute !== true || computing}
				title={canCompute === false || !compute
					? 'Calcul automatique indisponible : le moteur ne l’expose pas encore.'
					: 'Volumes calculés par le moteur à partir des couleurs (comme Orca)'}
				onclick={runCompute}>{computing ? 'Calcul…' : 'Calculer depuis les couleurs'}</button
			>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-ghost"
				onclick={() => {
					draft = resizeFlushMatrix(undefined, n);
					factor = 1;
				}}>Valeurs par défaut ({DEFAULT_FLUSH} mm³)</button
			>
		</div>
		{#if computeError}
			<p class="text-xs text-danger" role="alert">{computeError}</p>
		{:else if canCompute === false}
			<p class="text-xs text-content-subtle" data-testid="flush-compute-unavailable">
				Le calcul depuis les couleurs sera disponible quand le moteur l'exposera ; saisissez les
				volumes à la main en attendant.
			</p>
		{/if}
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>Annuler</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="flush-apply"
			onclick={() => {
				onapply(draft, factor > 0 ? factor : 1);
				onclose();
			}}>Appliquer</button
		>
	{/snippet}
</Dialog>
