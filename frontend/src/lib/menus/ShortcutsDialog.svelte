<script lang="ts">
	// Dialogue d'aide des raccourcis clavier (T079, Annexe B §B.6). Modal listant
	// les 92 raccourcis groupés, ouvert par « ? » ou le menu Aide. Présentational
	// seulement : les données viennent de `shortcuts.ts`.
	import { SHORTCUT_GROUPS, totalShortcuts } from './shortcuts';

	let { open = false, onClose }: { open?: boolean; onClose: () => void } = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

{#if open}
	<!-- Fond cliquable pour fermer. -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="max-h-[85vh] w-full max-w-3xl overflow-y-auto rounded-lg border border-slate-700 bg-slate-900 p-6 text-slate-200 shadow-xl"
			role="dialog"
			aria-modal="true"
			aria-label="Raccourcis clavier"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			tabindex="-1"
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-slate-100">
					Raccourcis clavier <span class="text-sm text-slate-500">({totalShortcuts()})</span>
				</h2>
				<button
					type="button"
					class="rounded bg-slate-700 px-2 py-1 text-sm hover:bg-slate-600"
					onclick={onClose}
				>
					Fermer
				</button>
			</div>

			<div class="grid gap-6 sm:grid-cols-2">
				{#each SHORTCUT_GROUPS as group (group.group)}
					<section aria-label={group.group}>
						<h3 class="mb-2 text-sm font-semibold text-sky-300">{group.group}</h3>
						<dl class="flex flex-col gap-1">
							{#each group.shortcuts as shortcut (shortcut.keys + shortcut.action)}
								<div class="flex items-start justify-between gap-3 text-xs">
									<dt class="text-slate-400">{shortcut.action}</dt>
									<dd>
										<kbd
											class="whitespace-nowrap rounded border border-slate-600 bg-slate-800 px-1.5 py-0.5 font-mono text-slate-200"
										>
											{shortcut.keys}
										</kbd>
									</dd>
								</div>
							{/each}
						</dl>
					</section>
				{/each}
			</div>
		</div>
	</div>
{/if}
