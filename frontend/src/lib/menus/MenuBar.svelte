<script lang="ts">
	// Barre de menus principale (T107–T111) : rend `MAIN_MENUS` (Fichier, Édition,
	// Vue, Calibration, Aide) en menus déroulants. Présentational : émet
	// l'identifiant d'action stable ; la page (`routes/projects/[id]`) le dispatche
	// vers les commandes réelles. Libellés localisés via `$t` (repli anglais).
	import { tick } from 'svelte';
	import type { Menu } from './menus';
	import { t } from '$lib/i18n';
	import { altLetter, menuForMnemonic, menuMnemonic, moveIndex } from './menu-keys';

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
	// Audit 48 D2 : barre « active » — un menu ouvert, puis survol d'un autre
	// titre ⇒ ouverture immédiate de celui-ci (menus natifs d'Orca/wx : on
	// parcourt les menus en glissant). Le premier menu s'ouvre toujours au clic
	// ou au clavier ; ce survol ne fait rien tant qu'aucun menu n'est ouvert.
	function hoverTitle(title: string) {
		if (!openTitle || openTitle === title) return;
		openTitle = title;
		openSub = null;
		onopen?.(openTitle);
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

	// — Audit 42 A2 : clavier (motif WAI-ARIA « menubar », menus natifs d'Orca) —
	// Titres : ↓ / Entrée / Espace ouvrent le menu sur son 1er élément, ↑ sur le
	// dernier, ← / → passent au titre voisin. Menu ouvert : ↑ / ↓ / Début / Fin
	// parcourent les éléments, → déplie un sous-menu (sinon menu suivant), ←
	// replie (sinon menu précédent), Échap / Tab referment et rendent le focus au
	// titre. Alt+lettre (première lettre du titre affiché) ouvre un menu.
	const titleEls: HTMLButtonElement[] = [];
	let menuEl = $state<HTMLDivElement | null>(null);
	const items = (): HTMLButtonElement[] =>
		menuEl ? Array.from(menuEl.querySelectorAll<HTMLButtonElement>(':scope > button')) : [];
	async function openAt(index: number, focus: 'first' | 'last' | 'none' = 'first') {
		const n = menus.length;
		if (n === 0) return;
		const menu = menus[((index % n) + n) % n];
		if (openTitle !== menu.title) {
			openTitle = menu.title;
			openSub = null;
			onopen?.(menu.title);
		}
		await tick();
		const list = items();
		if (focus === 'first') list[0]?.focus();
		else if (focus === 'last') list[list.length - 1]?.focus();
	}
	function closeToTitle() {
		const index = menus.findIndex((m) => m.title === openTitle);
		close();
		if (index >= 0) titleEls[index]?.focus();
	}
	function onTitleKey(e: KeyboardEvent, index: number) {
		const n = menus.length;
		// Touches de navigation consommées ici : pas de raccourci d'éditeur
		// (flèches = déplacer l'objet sélectionné).
		if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Enter', ' '].includes(e.key))
			e.stopPropagation();
		if (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			void openAt(index, 'first');
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			void openAt(index, 'last');
		} else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
			e.preventDefault();
			const next = (index + (e.key === 'ArrowRight' ? 1 : -1) + n) % n;
			titleEls[next]?.focus();
			if (openTitle) void openAt(next, 'none');
		}
	}
	async function onMenuKey(e: KeyboardEvent, index: number) {
		const list = items();
		const current = list.indexOf(document.activeElement as HTMLButtonElement);
		const focused = current >= 0 ? list[current] : null;
		if (['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key))
			e.stopPropagation();
		switch (e.key) {
			case 'ArrowDown':
			case 'ArrowUp':
			case 'Home':
			case 'End':
				e.preventDefault();
				list[moveIndex(current, list.length, e.key)]?.focus();
				break;
			case 'ArrowRight': {
				e.preventDefault();
				const sub = focused?.dataset.subTrigger;
				if (sub !== undefined) {
					openSubNow(sub);
					await tick();
					const after = items();
					after[after.indexOf(focused!) + 1]?.focus();
				} else void openAt(index + 1, 'first');
				break;
			}
			case 'ArrowLeft': {
				e.preventDefault();
				const parent = focused?.dataset.subOf;
				if (parent !== undefined) {
					openSub = null;
					await tick();
					items()
						.find((b) => b.dataset.subTrigger === parent)
						?.focus();
				} else void openAt(index - 1, 'first');
				break;
			}
			case 'Escape':
			case 'Tab':
				e.preventDefault();
				e.stopPropagation();
				closeToTitle();
				break;
		}
	}
	function onWindowKey(e: KeyboardEvent) {
		if (openTitle && e.key === 'Escape') {
			close();
			return;
		}
		const letter = altLetter(e);
		if (!letter) return;
		const index = menuForMnemonic(
			menus.map((m) => $t(m.title)),
			letter
		);
		if (index < 0) return;
		e.preventDefault();
		e.stopPropagation();
		titleEls[index]?.focus();
		void openAt(index, 'first');
	}
</script>

<div class="relative flex items-center">
	{#each menus as menu, mi (menu.title)}
		<div class="relative">
			<button
				bind:this={titleEls[mi]}
				aria-keyshortcuts={'Alt+' + menuMnemonic($t(menu.title)).toUpperCase()}
				onkeydown={(e) => onTitleKey(e, mi)}
				type="button"
				class="ws-btn ws-btn-ghost px-2 font-normal {openTitle === menu.title
					? 'bg-overlay text-content'
					: ''} {openTitle ? 'relative z-30' : ''}"
				aria-haspopup="menu"
				aria-expanded={openTitle === menu.title}
				onclick={() => toggle(menu.title)}
				onpointerenter={() => hoverTitle(menu.title)}
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
					tabindex="-1"
					bind:this={menuEl}
					onkeydown={(e) => void onMenuKey(e, mi)}
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
								data-sub-trigger={entry.label}
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
										data-sub-of={entry.label}
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
<svelte:window onkeydown={onWindowKey} />

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
