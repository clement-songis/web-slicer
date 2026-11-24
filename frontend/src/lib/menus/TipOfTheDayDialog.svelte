<script lang="ts">
	import { dialogFocus } from '#lib/ui/dialog-focus';
	import { t } from '#lib/i18n';
	// Conseil du jour d'Orca (Help › Show Tip of the Day), adapté à un dialogue
	// web autonome. Le bouton Suivant change réellement le conseil affiché.
	let { open = false, onClose }: { open?: boolean; onClose: () => void } = $props();
	const tips = [
		'Use Ctrl+Z and Ctrl+Y to undo or redo a scene change.',
		'Duplicate the current plate from the Edit menu to compare two preparations.',
		'The Top, Front and Left views are available with Ctrl+1, Ctrl+3 and Ctrl+5.',
		'Save the project before slicing to keep the whole scene.'
	];
	let index = $state(0);
	$effect(() => {
		if (open) index = new Date().getDate() % tips.length;
	});
</script>

<svelte:window onkeydown={(event) => open && event.key === 'Escape' && onClose()} />

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4" role="presentation">
		<div
			class="ws-dialog w-full max-w-lg p-6"
			role="dialog"
			aria-modal="true"
			aria-label={$t('Tip of the day')}
			data-testid="tip-of-the-day"
			tabindex="-1"
			use:dialogFocus
		>
			<h2 class="ws-dialog-title mb-4 text-primary">{$t('Tip of the day')}</h2>
			<p class="min-h-16 text-sm leading-6 text-content-muted" data-testid="tip-text">
				{$t(tips[index])}
			</p>
			<div class="mt-6 flex justify-end gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					onclick={() => (index = (index + 1) % tips.length)}
				>
					{$t('Next')}
				</button>
				<button type="button" class="ws-btn ws-btn-primary" onclick={onClose}>{$t('Close')}</button>
			</div>
		</div>
	</div>
{/if}
