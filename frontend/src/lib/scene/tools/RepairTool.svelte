<script lang="ts">
	// Outil de réparation (T055) : analyse le maillage d'un modèle via l'endpoint
	// T054 et affiche le rapport (FR-012). L'appel réseau passe par le client API.
	import { ApiError } from '../../api/client';
	import { repairModel } from '../../api/scene';
	import type { RepairResponse } from '../../api/types';
	import { t, tr } from '#lib/i18n';

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
			error = e instanceof ApiError ? e.message : tr('analysis failed');
			report = null;
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	<button
		type="button"
		class="ws-btn ws-btn-primary self-start"
		disabled={loading}
		onclick={analyze}
	>
		{loading ? $t('Analyzing…') : $t('Analyze mesh')}
	</button>

	{#if error}
		<p class="text-danger" role="alert">{error}</p>
	{/if}

	{#if report}
		<dl class="grid grid-cols-2 gap-x-4 gap-y-1">
			<dt class="text-content-muted">{$t('Triangles')}</dt>
			<dd>{report.triangles}</dd>
			<dt class="text-content-muted">{$t('Degenerate facets')}</dt>
			<dd>{report.degenerate}</dd>
			<dt class="text-content-muted">{$t('Open edges')}</dt>
			<dd>{report.open_edges}</dd>
			<dt class="text-content-muted">{$t('Watertight')}</dt>
			<dd class={report.watertight ? 'text-success' : 'text-warning'}>
				{report.watertight ? $t('yes') : $t('no')}
			</dd>
		</dl>
	{/if}
</div>
