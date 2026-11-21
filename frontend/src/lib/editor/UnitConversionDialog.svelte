<script lang="ts">
	// Objet minuscule à l'import (Orca : « Object too small », Plater.cpp:6693) :
	// propose pouces → mm ou mètres → mm, avec la taille résultante de chaque
	// choix, l'unité détectée mise en avant, et « Ne plus proposer ».
	import { dialogFocus } from '$lib/ui/dialog-focus';
	import { locale, t } from '$lib/i18n';
	import { UNIT_FACTORS, formatSize, type UnitSource } from './units';

	interface Props {
		filename: string;
		/** Dimensions actuelles (mm). */
		size: [number, number, number];
		suggested: UnitSource;
		onconvert: (unit: UnitSource) => void;
		onkeep: (dontAskAgain: boolean) => void;
	}

	let { filename, size, suggested, onconvert, onkeep }: Props = $props();
	let dontAsk = $state(false);

	// Titres : clés anglaises traduites au rendu.
	const OPTIONS: { unit: UnitSource; title: string }[] = [
		{ unit: 'inch', title: 'The file is in inches' },
		{ unit: 'meter', title: 'The file is in meters' }
	];
	const ordered = $derived(
		[...OPTIONS].sort((a, b) => Number(b.unit === suggested) - Number(a.unit === suggested))
	);
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-scrim" role="presentation">
	<div
		class="ws-dialog w-[30rem] max-w-[calc(100vw-2rem)] p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="unit-conv-title"
		data-testid="unit-conversion-dialog"
		data-suggested={suggested}
		tabindex="-1"
		use:dialogFocus
		onkeydown={(e) => e.key === 'Escape' && onkeep(dontAsk)}
	>
		<h2 id="unit-conv-title" class="ws-dialog-title mb-1">{$t('Very small object')}</h2>
		<p class="text-sm text-content-muted">
			{$t('“{file}” measures only', { file: filename })}
			<strong class="text-content">{$locale && formatSize(size)}</strong>{$t(
				'. It was probably exported in a unit other than the millimeter.'
			)}
		</p>
		<div class="mt-3 flex flex-col gap-2">
			{#each ordered as o (o.unit)}
				<button
					type="button"
					class="flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-left {o.unit ===
					suggested
						? 'border-primary bg-primary/5'
						: 'border-border-strong hover:bg-overlay'}"
					data-testid="unit-convert-{o.unit}"
					onclick={() => onconvert(o.unit)}
				>
					<span>
						<span class="block text-sm font-medium text-content">{$t(o.title)}</span>
						<span class="block text-xs text-content-subtle">
							×{String(UNIT_FACTORS[o.unit]).replace('.', $locale === 'fr' ? ',' : '.')} → {formatSize(
								size.map((v) => v * UNIT_FACTORS[o.unit])
							)}
						</span>
					</span>
					{#if o.unit === suggested}<span class="ws-badge">{$t('Likely')}</span>{/if}
				</button>
			{/each}
		</div>
		<p class="mt-2 text-xs text-content-subtle">
			{$t('Reversible: right-click the object, then “Restore to inches” (or meters).')}
		</p>
		<div class="mt-4 flex items-center justify-between gap-3">
			<label class="flex items-center gap-2 text-xs text-content-muted">
				<input type="checkbox" bind:checked={dontAsk} data-testid="unit-dont-ask" />
				{$t("Don't suggest again")}
			</label>
			<button
				type="button"
				class="ws-btn ws-btn-secondary"
				data-testid="unit-keep"
				onclick={() => onkeep(dontAsk)}>{$t('Keep in millimeters')}</button
			>
		</div>
	</div>
</div>
