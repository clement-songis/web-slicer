<script lang="ts">
	// Barre de menus principale (T107–T111) : rend `MAIN_MENUS` (Fichier, Édition,
	// Vue, Calibration, Aide) en menus déroulants. Présentational : émet
	// l'identifiant d'action stable ; la page (`routes/projects/[id]`) le dispatche
	// vers les commandes réelles. Libellés localisés via `$t` (repli anglais).
	import type { Menu } from './menus';
	import { t } from '$lib/i18n';

	interface Props {
		menus: Menu[];
		onaction: (action: string) => void;
		/** Menu ouvert (titre) : la page peut rafraîchir un contenu dynamique (récents). */
		onopen?: (title: string) => void;
	}

	let { menus, onaction, onopen }: Props = $props();

	// Menu ouvert (par titre) et sous-menu déplié (par libellé), locaux à la barre.
	let openTitle = $state<string | null>(null);
	let openSub = $state<string | null>(null);

	function toggle(title: string) {
		openTitle = openTitle === title ? null : title;
		openSub = null;
		if (openTitle) onopen?.(openTitle);
	}
	function close() {
		openTitle = null;
		openSub = null;
	}
	function pick(action: string) {
		onaction(action);
		close();
	}
	// Audit 40 D2 : sous-menu déplié au survol (après un court délai, comme les
	// menus natifs d'Orca) ; le clic reste possible (tactile, clavier). Un clic
	// juste après l'ouverture au survol ne le referme pas.
	const HOVER_OPEN_MS = 120;
	let hoverTimer: ReturnType<typeof setTimeout> | undefined;
	let openedAt = 0;
	function openSubNow(label: string) {
		if (openSub !== label) openedAt = Date.now();
		openSub = label;
	}
	function hoverSub(label: string) {
		clearTimeout(hoverTimer);
		hoverTimer = setTimeout(() => openSubNow(label), HOVER_OPEN_MS);
	}
	function clickSub(label: string) {
		clearTimeout(hoverTimer);
		if (openSub === label && Date.now() - openedAt > 400) openSub = null;
		else openSubNow(label);
	}
</script>

<div class="relative flex items-center">
	{#each menus as menu (menu.title)}
		<div class="relative">
			<button
				type="button"
				class="ws-btn ws-btn-ghost px-2 font-normal {openTitle === menu.title
					? 'bg-overlay text-content'
					: ''}"
				aria-haspopup="menu"
				aria-expanded={openTitle === menu.title}
				onclick={() => toggle(menu.title)}
			>
				{$t(menu.title)}
			</button>

			{#if openTitle === menu.title}
				<div
					class="absolute top-full left-0 z-30 mt-1 max-h-[calc(100vh-4rem)] max-w-[26rem] overflow-y-auto ws-menu {menu.entries.some(
						(e) => e !== 'separator' && e.hint
					)
						? 'w-80'
						: 'min-w-56'}"
					role="menu"
				>
					{#each menu.entries as entry, i (i)}
						{#if entry === 'separator'}
							<div class="my-1 h-px bg-border"></div>
						{:else if entry.items}
							<!-- Sous-menu (Import/Export) : dépliable en place. -->
							<button
								type="button"
								class="ws-menu-item justify-between"
								aria-haspopup="menu"
								aria-expanded={openSub === entry.label}
								onpointerenter={() => hoverSub(entry.label)}
								onpointerleave={() => clearTimeout(hoverTimer)}
								onclick={() => clickSub(entry.label)}
							>
								<span>{$t(entry.label)}</span>
								<span class="text-content-subtle">{openSub === entry.label ? '▾' : '▸'}</span>
							</button>
							{#if openSub === entry.label}
								{#each entry.items as sub (sub.action)}
									<button
										type="button"
										class="ws-menu-item justify-between pl-6"
										role="menuitem"
										onclick={() => pick(sub.action)}
									>
										<span class={sub.raw ? 'truncate' : ''}
											>{sub.raw ? sub.label : $t(sub.label)}</span
										>
										{#if sub.shortcut}<span class="ml-4 text-xs text-content-subtle"
												>{sub.shortcut}</span
											>{/if}
									</button>
								{/each}
							{/if}
						{:else}
							<button
								type="button"
								class="ws-menu-item justify-between {entry.hint ? 'flex-wrap gap-y-0' : ''}"
								role="menuitem"
								aria-describedby={entry.hint ? `menu-hint-${entry.action}` : undefined}
								onclick={() => pick(entry.action)}
							>
								<span>{$t(entry.label)}</span>
								{#if entry.shortcut}<span class="ml-4 text-xs text-content-subtle"
										>{entry.shortcut}</span
									>{/if}
								<!-- Description (aide d'Orca) : hors du nom accessible, qui reste le
								     libellé de parité ; lue comme description (aria-describedby). -->
								{#if entry.hint}<span
										id="menu-hint-{entry.action}"
										aria-hidden="true"
										class="block w-full text-left text-xs leading-tight text-content-subtle"
										data-testid="menu-hint">{$t(entry.hint)}</span
									>{/if}
							</button>
						{/if}
					{/each}
				</div>
			{/if}
		</div>
	{/each}
</div>

<!-- Échap referme le menu déroulant ouvert (comportement natif des barres de menus). -->
<svelte:window onkeydown={(e) => openTitle && e.key === 'Escape' && close()} />

<!-- Fond transparent : referme la barre au clic extérieur. -->
{#if openTitle}
	<button
		type="button"
		class="fixed inset-0 z-20 cursor-default"
		tabindex="-1"
		aria-label={$t('Close menu')}
		onclick={close}
	></button>
{/if}
