<script lang="ts">
	// Annuler / Rétablir avec la liste des étapes (Orca : liste déroulante de
	// l'historique d'annulation, GLCanvas3D::_render_undo_redo_stack) : choisir
	// une étape annule (ou rétablit) jusqu'à elle comprise.
	import IconUndo from '~icons/lucide/undo-2';
	import IconRedo from '~icons/lucide/redo-2';
	import IconChevron from '~icons/lucide/chevron-down';
	import { t } from '$lib/i18n';

	interface Props {
		/** Lit les étapes (`undo[0]` = prochaine annulation). */
		entries: () => { undo: string[]; redo: string[] };
		/** Change à chaque modification de l'historique (rafraîchit l'état). */
		version: number;
		onundo: (steps: number) => void;
		onredo: (steps: number) => void;
		/** Enregistre l'état en attente (debounce) avant d'afficher la liste. */
		onflush?: () => void;
	}
	let { entries, version, onundo, onredo, onflush }: Props = $props();

	// Lecture non réactive de l'historique : recalculée à chaque `version`.
	const list = $derived.by(() => {
		void version;
		void $t; // libellés d'étapes traduits à la lecture : relus au changement de langue
		return entries();
	});
	let open = $state<'undo' | 'redo' | null>(null);
	let hover = $state(-1);
	let root = $state<HTMLDivElement | null>(null);

	function toggle(which: 'undo' | 'redo') {
		if (open !== which) onflush?.();
		open = open === which ? null : which;
		hover = -1;
	}
	function pick(steps: number) {
		const which = open;
		open = null;
		if (which === 'undo') onundo(steps);
		else if (which === 'redo') onredo(steps);
	}
	function onWindowPointer(e: PointerEvent) {
		if (open && root && !root.contains(e.target as Node)) open = null;
	}
</script>

<svelte:window onpointerdown={onWindowPointer} />

<div
	class="ws-floating relative flex items-center gap-0.5 p-1"
	bind:this={root}
	data-testid="undo-history"
>
	{#each [['undo', 'Undo', IconUndo], ['redo', 'Redo', IconRedo]] as const as [which, label, Icon] (which)}
		{@const items = list[which]}
		<div class="flex items-center">
			<button
				type="button"
				class="ws-icon-btn ws-icon-btn-sm"
				aria-label={items.length
					? $t(which === 'undo' ? 'Undo: {step}' : 'Redo: {step}', { step: items[0] })
					: $t(label)}
				data-tip={items.length
					? $t(which === 'undo' ? 'Undo: {step} ({key})' : 'Redo: {step} ({key})', {
							step: items[0],
							key: which === 'undo' ? 'Ctrl+Z' : 'Ctrl+Y'
						})
					: $t(label)}
				data-testid={`history-${which}`}
				disabled={!items.length}
				onclick={() => (which === 'undo' ? onundo(1) : onredo(1))}
				><Icon class="h-4 w-4" aria-hidden="true" /></button
			>
			<button
				type="button"
				class="ws-icon-btn ws-icon-btn-xs"
				aria-label={$t(
					which === 'undo' ? 'History: undo several steps' : 'History: redo several steps'
				)}
				aria-expanded={open === which}
				aria-haspopup="menu"
				data-testid={`history-${which}-list`}
				disabled={!items.length}
				onclick={() => toggle(which)}><IconChevron class="h-3 w-3" aria-hidden="true" /></button
			>
		</div>
	{/each}
	{#if open}
		{@const items = list[open]}
		<div
			class="ws-floating absolute top-full left-0 z-20 mt-1 max-h-80 w-72 overflow-y-auto p-1"
			role="menu"
			aria-label={open === 'undo' ? $t('Steps to undo') : $t('Steps to redo')}
			data-testid="history-menu"
		>
			{#each items as label, i (i)}
				<button
					type="button"
					role="menuitem"
					class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm {i <= hover
						? 'bg-primary/10 text-primary'
						: 'hover:bg-overlay'}"
					data-testid="history-step"
					onpointerenter={() => (hover = i)}
					onfocus={() => (hover = i)}
					onclick={() => pick(i + 1)}
					><span class="w-5 shrink-0 text-right text-xs text-content-subtle tabular-nums"
						>{i + 1}</span
					><span class="truncate">{label}</span></button
				>
			{/each}
			<p class="px-2 pt-1 text-xs text-content-subtle" aria-live="polite">
				{hover >= 0
					? $t(open === 'undo' ? 'Undo {n} step(s)' : 'Redo {n} step(s)', { n: hover + 1 })
					: $t('{n} step(s)', { n: items.length })}
			</p>
		</div>
	{/if}
</div>
