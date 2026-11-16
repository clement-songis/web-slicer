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
	}

	let { menus, onaction }: Props = $props();

	// Menu ouvert (par titre) et sous-menu déplié (par libellé), locaux à la barre.
	let openTitle = $state<string | null>(null);
	let openSub = $state<string | null>(null);

	function toggle(title: string) {
		openTitle = openTitle === title ? null : title;
		openSub = null;
	}
	function close() {
		openTitle = null;
		openSub = null;
	}
	function pick(action: string) {
		onaction(action);
		close();
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
				<div class="absolute top-full left-0 z-30 mt-1 min-w-56 ws-menu" role="menu">
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
								onclick={() => (openSub = openSub === entry.label ? null : entry.label)}
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
										<span>{$t(sub.label)}</span>
										{#if sub.shortcut}<span class="ml-4 text-xs text-content-subtle"
												>{sub.shortcut}</span
											>{/if}
									</button>
								{/each}
							{/if}
						{:else}
							<button
								type="button"
								class="ws-menu-item justify-between"
								role="menuitem"
								onclick={() => pick(entry.action)}
							>
								<span>{$t(entry.label)}</span>
								{#if entry.shortcut}<span class="ml-4 text-xs text-content-subtle"
										>{entry.shortcut}</span
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
		aria-label="Fermer le menu"
		onclick={close}
	></button>
{/if}
