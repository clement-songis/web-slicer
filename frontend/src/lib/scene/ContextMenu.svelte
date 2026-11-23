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

	// Menu rangé en sous-menus ouverts (sections titrées, phase 25) : rendu en
	// colonnes pour tenir à l'écran sans défilement.
	const sectioned = $derived(items.some((item) => item !== 'separator' && item.section));
</script>

{#if open}
	<!-- Fond de fermeture (clic extérieur / clic droit). -->
	<button
		type="button"
		class="fixed inset-0 z-40 cursor-default"
		tabindex="-1"
		aria-label={$t('Close the menu')}
		onclick={onclose}
		oncontextmenu={(e) => {
			e.preventDefault();
			onclose();
		}}
	></button>
	<div
		bind:this={menuElement}
		class="ws-menu fixed z-50 max-h-[calc(100vh-1rem)] max-w-[calc(100vw-1rem)] min-w-48 overflow-y-auto overscroll-contain {sectioned
			? 'w-[min(46rem,calc(100vw-1rem))] columns-[13.5rem] gap-1'
			: ''}"
		style="left: {left}px; top: {top}px"
		role="menu"
		data-layout={sectioned ? 'sections' : 'list'}
	>
		{#each items as item, i (i)}
			{#if item === 'separator'}
				<div class="my-1 h-px bg-border"></div>
			{:else if item.section && item.items}
				<!-- Sous-menu ouvert : titre + actions, jamais coupé entre deux colonnes. -->
				<div
					role="group"
					aria-labelledby="ctx-section-{item.action}"
					class="break-inside-avoid pb-1"
					data-testid="menu-section-{item.action}"
				>
					<div
						id="ctx-section-{item.action}"
						class="px-3 pt-1.5 pb-0.5 text-[11px] font-semibold tracking-wide text-content-subtle uppercase"
					>
						{$t(item.label)}
					</div>
					{#each item.items as sub, j (j)}
						{#if sub.split && sub.items}
							<!-- Action + sous-menu (audit 43 M1-a) : libellé = action d'origine,
							     flèche = formes d'Orca (Charger…, Cube… SVG), dépliées sur place. -->
							<div class="flex items-stretch" data-testid="menu-split-{sub.action}">
								<!-- Clavier : → déplie les formes, ← les replie (menus natifs). -->
								<button
									type="button"
									class="ws-menu-item min-w-0 flex-1"
									role="menuitem"
									aria-haspopup="menu"
									aria-expanded={expanded === sub.action}
									data-action={sub.action}
									onclick={() => pick(sub.action)}
									onkeydown={(e) => {
										if (
											(e.key === 'ArrowRight' && expanded !== sub.action) ||
											(e.key === 'ArrowLeft' && expanded === sub.action)
										) {
											e.preventDefault();
											toggle(sub.action);
										}
									}}
								>
									{$t(sub.label)}
								</button>
								<!-- Flèche à la souris : bouton hors de la liste des entrées (tabindex -1). -->
								<button
									type="button"
									tabindex="-1"
									class="ws-menu-item w-8 shrink-0 text-center text-content-subtle"
									aria-label={$t('Shapes')}
									title={$t('Shapes')}
									data-testid="menu-sub-{sub.action}"
									onclick={() => toggle(sub.action)}
								>
									{expanded === sub.action ? '▾' : '▸'}
								</button>
							</div>
							{#if expanded === sub.action}
								<div
									role="group"
									aria-label={$t(sub.label)}
									class="ml-2 border-l border-border pl-1"
									data-testid="menu-shapes-{sub.action}"
								>
									{#each sub.items as leaf, k (leaf.action)}
										{#if k === 1}<div class="my-1 h-px bg-border"></div>{/if}
										<button
											type="button"
											class="ws-menu-item"
											role="menuitem"
											data-action={leaf.action}
											onclick={() => pick(leaf.action)}
										>
											{$t(leaf.label)}
										</button>
									{/each}
								</div>
							{/if}
						{:else}
							<button
								type="button"
								class="ws-menu-item flex items-baseline justify-between gap-3"
								role="menuitem"
								data-action={sub.action}
								onclick={() => pick(sub.action)}
							>
								<span>{$t(sub.label)}</span>
								{#if sub.hint && sub.hint !== item.label}<span class="text-xs text-content-subtle">
										{$t(sub.hint)}
									</span>{/if}
							</button>
						{/if}
					{/each}
				</div>
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
					<span aria-hidden="true" class="text-content-subtle">
						{expanded === item.action ? '▾' : '▸'}
					</span>
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
					class="ws-menu-item {item.hint ? 'flex items-baseline justify-between gap-3' : ''}"
					role="menuitem"
					onclick={() => pick(item.action)}
				>
					{#if item.hint}
						<span>{$t(item.label)}</span>
						<span class="text-xs text-content-subtle">{$t(item.hint)}</span>
					{:else}
						{$t(item.label)}
					{/if}
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
