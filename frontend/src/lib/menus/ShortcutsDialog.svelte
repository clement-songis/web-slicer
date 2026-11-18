<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Dialogue d'aide des raccourcis clavier (T079, Annexe B §B.6). Modal listant
	// les 92 raccourcis groupés, ouvert par « ? » ou le menu Aide. Présentational
	// seulement : les données viennent de `shortcuts.ts`.
	import { locale, t } from '$lib/i18n';
	import { SHORTCUT_GROUPS, totalShortcuts } from './shortcuts';
	import { shortcutKeysText, shortcutText } from './shortcuts-fr';

	let { open = false, onClose }: { open?: boolean; onClose: () => void } = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={open ? onKeydown : undefined} />

{#if open}
	<!-- Fond cliquable pour fermer. -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="ws-dialog max-h-[85vh] w-full max-w-3xl overflow-y-auto p-6"
			role="dialog"
			aria-modal="true"
			aria-label="Raccourcis clavier"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			tabindex="-1"
			use:dialogFocus={onClose}
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 class="ws-dialog-title">
					{$t('Keyboard Shortcuts')}
					<span class="text-sm text-content-subtle">({totalShortcuts()})</span>
				</h2>
				<button type="button" class="ws-btn ws-btn-secondary" onclick={onClose}>
					{$t('Close')}
				</button>
			</div>

			<div class="grid gap-6 sm:grid-cols-2">
				{#each SHORTCUT_GROUPS as group (group.group)}
					<section aria-label={shortcutText(group.group, $locale)}>
						<h3 class="mb-2 text-sm font-semibold text-primary">
							{shortcutText(group.group, $locale)}
						</h3>
						<dl class="flex flex-col gap-1">
							{#each group.shortcuts as shortcut (shortcut.keys + shortcut.action)}
								<div class="flex items-start justify-between gap-3 text-xs">
									<dt class="text-content-muted">{shortcutText(shortcut.action, $locale)}</dt>
									<dd>
										<kbd
											class="whitespace-nowrap rounded border border-border-strong bg-surface-sunken px-1.5 py-0.5 font-mono text-content"
										>
											{shortcutKeysText(shortcut.keys, $locale)}
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
