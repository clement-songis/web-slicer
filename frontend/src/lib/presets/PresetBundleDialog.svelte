<script lang="ts">
	import { dialogFocus } from '#lib/ui/dialog-focus';
	import { t } from '#lib/i18n';
	// Adaptation web de PresetBundleDialog : le lot actif du projet et ses
	// profils, exportables dans une archive de JSON Orca.
	export interface BundlePresetRef {
		id: string;
		kind: 'machine' | 'process' | 'filament';
		name: string;
	}
	interface Props {
		open: boolean;
		projectName: string;
		presets: readonly BundlePresetRef[];
		busy?: boolean;
		error?: string | null;
		onclose: () => void;
		onexport: () => void;
		/** Importe des profils / un lot Orca (.json, .zip) : même action que
		 *  Fichier › Importer › Importer des configurations. */
		onimport?: () => void;
	}
	let {
		open,
		projectName,
		presets,
		busy = false,
		error = null,
		onclose,
		onexport,
		onimport
	}: Props = $props();
	const labels = { machine: 'Printer', process: 'Process||kind', filament: 'Filament' };
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={() => !busy && onclose()}
	>
		<div
			class="ws-dialog w-full max-w-lg p-5"
			role="dialog"
			aria-modal="true"
			aria-label={$t('Preset bundle')}
			data-testid="preset-bundle-dialog"
			tabindex="-1"
			use:dialogFocus
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && !busy && onclose()}
		>
			<h2 class="ws-dialog-title">{$t('Preset bundle')}</h2>
			<p class="mt-1 text-sm text-content-muted">
				{$t('Active bundle of project “{name}”', { name: projectName })}
			</p>
			<div
				class="mt-4 divide-y divide-border rounded border border-border"
				data-testid="preset-bundle-list"
			>
				{#each presets as preset (`${preset.kind}-${preset.id}`)}
					<div
						class="flex items-center justify-between gap-4 px-3 py-2 text-sm"
						data-testid={`bundle-${preset.kind}`}
					>
						<span class="text-content-muted">{$t(labels[preset.kind])}</span>
						<span class="text-right font-medium">{preset.name}</span>
					</div>
				{/each}
			</div>
			{#if error}<p class="mt-3 text-sm text-danger" role="alert">{error}</p>{/if}
			<div class="mt-5 flex justify-end gap-2">
				<button type="button" class="ws-btn ws-btn-secondary" disabled={busy} onclick={onclose}>
					{$t('Close')}
				</button>
				{#if onimport}
					<button
						type="button"
						data-testid="import-preset-bundle"
						class="ws-btn ws-btn-secondary"
						disabled={busy}
						onclick={onimport}
					>
						{$t('Import…')}
					</button>
				{/if}
				<button
					type="button"
					data-testid="export-preset-bundle"
					class="ws-btn ws-btn-primary"
					disabled={busy || presets.length === 0}
					onclick={onexport}
				>
					{busy ? $t('Exporting…') : $t('Export presets')}
				</button>
			</div>
		</div>
	</div>
{/if}
