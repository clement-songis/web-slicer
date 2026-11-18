<script lang="ts">
	// Menu contextuel objet (T112) : positionné au curseur, liste les actions objet
	// (`OBJECT_CONTEXT_ITEMS`). Présentational : émet l'identifiant d'action, la page
	// l'applique sur l'objet ciblé. Fond transparent → ferme au clic extérieur.
	import type { ObjectMenuEntry } from './menus';
	import { t } from '$lib/i18n';

	interface Props {
		open: boolean;
		x: number;
		y: number;
		items: ObjectMenuEntry[];
		onaction: (action: string) => void;
		onclose: () => void;
	}

	let { open, x, y, items, onaction, onclose }: Props = $props();

	const MARGIN = 8;
	let menuElement: HTMLDivElement | null = null;
	let left = $state(x);
	let top = $state(y);

	// Mesure le menu rendu puis le retourne vers l'intérieur du viewport. La
	// hauteur CSS est bornée ; une liste plus longue que l'écran défile donc au
	// lieu de rendre ses dernières actions inaccessibles.
	function reposition() {
		if (!open || !menuElement) return;
		const rect = menuElement.getBoundingClientRect();
		const maxLeft = Math.max(MARGIN, window.innerWidth - rect.width - MARGIN);
		const maxTop = Math.max(MARGIN, window.innerHeight - rect.height - MARGIN);
		left = Math.max(MARGIN, Math.min(x, maxLeft));
		top = Math.max(MARGIN, Math.min(y, maxTop));
	}

	$effect(() => {
		if (!open) return;
		// Les props sont lues par reposition pendant l'effet : ouverture, nouveau
		// clic et changement de contenu recalculent ainsi la position.
		reposition();
	});

	function pick(action: string) {
		onaction(action);
		onclose();
	}

	// Sous-menus dépliés sur place (accordéon) : lisibles et utilisables au
	// clavier, sans menu volant qui sortirait de l'écran.
	let expanded = $state<string | null>(null);
	$effect(() => {
		if (!open) expanded = null;
	});
	function toggle(action: string) {
		expanded = expanded === action ? null : action;
		queueMicrotask(reposition);
	}
</script>

{#if open}
	<!-- Fond de fermeture (clic extérieur / clic droit). -->
	<button
		type="button"
		class="fixed inset-0 z-40 cursor-default"
		tabindex="-1"
		aria-label="Fermer le menu"
		onclick={onclose}
		oncontextmenu={(e) => {
			e.preventDefault();
			onclose();
		}}
	></button>
	<div
		bind:this={menuElement}
		class="fixed z-50 max-h-[calc(100vh-1rem)] min-w-48 max-w-[calc(100vw-1rem)] overflow-y-auto overscroll-contain ws-menu"
		style="left: {left}px; top: {top}px"
		role="menu"
	>
		{#each items as item, i (i)}
			{#if item === 'separator'}
				<div class="my-1 h-px bg-border"></div>
			{:else if item.items}
				<button
					type="button"
					class="ws-menu-item flex items-center justify-between gap-3"
					role="menuitem"
					aria-haspopup="true"
					aria-expanded={expanded === item.action}
					data-testid="menu-sub-{item.action}"
					onclick={() => toggle(item.action)}
				>
					<span>{$t(item.label)}</span>
					<span aria-hidden="true" class="text-content-subtle"
						>{expanded === item.action ? '▾' : '▸'}</span
					>
				</button>
				{#if expanded === item.action}
					<div role="group" aria-label={$t(item.label)} class="ml-2 border-l border-border pl-1">
						{#each item.items as sub (sub.action)}
							<button
								type="button"
								class="ws-menu-item flex items-baseline justify-between gap-3"
								role="menuitem"
								onclick={() => pick(sub.action)}
							>
								<span>{$t(sub.label)}</span>
								{#if sub.hint}<span class="text-xs text-content-subtle">{sub.hint}</span>{/if}
							</button>
						{/each}
					</div>
				{/if}
			{:else}
				<button
					type="button"
					class="ws-menu-item"
					role="menuitem"
					onclick={() => pick(item.action)}
				>
					{$t(item.label)}
				</button>
			{/if}
		{/each}
	</div>
{/if}

<!-- Échap referme le menu, comme les menus contextuels natifs d'Orca (wxMenu). -->
<svelte:window
	onresize={reposition}
	onkeydown={(e) => {
		if (open && e.key === 'Escape') onclose();
	}}
/>
