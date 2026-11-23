<script lang="ts">
	// Ordre des filaments glissable (Orca DragCanvas, PlateSettingsDialog) :
	// pastilles numérotées aux couleurs des filaments, réordonnées au glisser-
	// déposer ou au clavier (← / →, boutons). Présentationnel.
	import { swatchStyle } from '$lib/ui/color';
	import { t } from '$lib/i18n';
	import { moveItem } from './plate-sequence';

	interface Props {
		/** Numéros de filament (1-based) dans l'ordre d'impression. */
		order: number[];
		colours: readonly string[];
		onchange: (order: number[]) => void;
		label: string;
		testid?: string;
	}

	let { order, colours, onchange, label, testid = 'filament-order' }: Props = $props();

	let dragging = $state<number | null>(null);

	function move(from: number, to: number) {
		if (to < 0 || to >= order.length) return;
		onchange(moveItem(order, from, to));
	}
</script>

<ol class="flex flex-wrap items-center gap-1" aria-label={label} data-testid={testid}>
	{#each order as filament, i (filament)}
		<li
			class="flex items-center rounded ring-1 ring-border-strong {dragging === i
				? 'opacity-50'
				: ''}"
			draggable="true"
			data-filament={filament}
			ondragstart={(e) => {
				dragging = i;
				e.dataTransfer?.setData('text/plain', String(i));
			}}
			ondragend={() => (dragging = null)}
			ondragover={(e) => e.preventDefault()}
			ondrop={(e) => {
				e.preventDefault();
				const from = dragging ?? Number(e.dataTransfer?.getData('text/plain'));
				dragging = null;
				if (Number.isInteger(from)) move(from, i);
			}}
		>
			<button
				type="button"
				class="px-0.5 text-xs text-content-subtle disabled:opacity-30"
				aria-label={$t('Move filament {n} earlier', { n: filament })}
				disabled={i === 0}
				onclick={() => move(i, i - 1)}
			>
				‹
			</button>
			<span
				class="inline-flex h-6 w-7 cursor-grab items-center justify-center rounded text-xs font-semibold"
				style={swatchStyle(colours[filament - 1] ?? '#cccccc')}
				title={$t('Filament {n}', { n: filament })}
				tabindex="0"
				role="button"
				aria-label={$t('Filament {n}', { n: filament })}
				onkeydown={(e) => {
					if (e.key === 'ArrowLeft') {
						e.preventDefault();
						move(i, i - 1);
					} else if (e.key === 'ArrowRight') {
						e.preventDefault();
						move(i, i + 1);
					}
				}}
			>
				{filament}
			</span>
			<button
				type="button"
				class="px-0.5 text-xs text-content-subtle disabled:opacity-30"
				aria-label={$t('Move filament {n} later', { n: filament })}
				disabled={i === order.length - 1}
				onclick={() => move(i, i + 1)}
			>
				›
			</button>
		</li>
	{/each}
</ol>
