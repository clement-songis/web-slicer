<script lang="ts">
	// Outil de réparation (T055) : analyse le maillage d'un modèle via l'endpoint
	// T054 et affiche le rapport (FR-012). L'appel réseau passe par le client API.
	import { ApiError } from '../../api/client';
	import { repairModel } from '../../api/scene';
	import type { RepairResponse } from '../../api/types';

	interface Props {
		modelId: string;
	}

	let { modelId }: Props = $props();

	let loading = $state(false);
	let report = $state<RepairResponse | null>(null);
	let error = $state<string | null>(null);

	async function analyze() {
		loading = true;
		error = null;
		try {
			report = await repairModel(modelId);
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'échec de l’analyse';
			report = null;
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<button
		type="button"
		class="self-start rounded bg-sky-600 px-3 py-1 text-white hover:bg-sky-500 disabled:opacity-50"
		disabled={loading}
		onclick={analyze}>{loading ? 'Analyse…' : 'Analyser le maillage'}</button
	>

	{#if error}
		<p class="text-red-400" role="alert">{error}</p>
	{/if}

	{#if report}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-slate-400">Triangles</dt>
			<dd>{report.triangles}</dd>
			<dt class="text-slate-400">Facettes dégénérées</dt>
			<dd>{report.degenerate}</dd>
			<dt class="text-slate-400">Arêtes de bord</dt>
			<dd>{report.open_edges}</dd>
			<dt class="text-slate-400">Étanche</dt>
			<dd class={report.watertight ? 'text-green-400' : 'text-amber-400'}>
				{report.watertight ? 'oui' : 'non'}
			</dd>
		</dl>
	{/if}
</div>
