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

	function pick(action: string) {
		onaction(action);
		onclose();
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
		class="fixed z-50 min-w-48 rounded border border-border bg-surface-raised py-1 shadow-xl"
		style="left: {x}px; top: {y}px"
		role="menu"
	>
		{#each items as item, i (i)}
			{#if item === 'separator'}
				<div class="my-1 h-px bg-border"></div>
			{:else}
				<button
					type="button"
					class="block w-full px-3 py-1.5 text-left text-sm text-content-muted hover:bg-overlay hover:text-content"
					role="menuitem"
					onclick={() => pick(item.action)}
				>
					{$t(item.label)}
				</button>
			{/if}
		{/each}
	</div>
{/if}
