<script lang="ts">
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Conseil du jour d'Orca (Help › Show Tip of the Day), adapté à un dialogue
	// web autonome. Le bouton Suivant change réellement le conseil affiché.
	let { open = false, onClose }: { open?: boolean; onClose: () => void } = $props();
	const tips = [
		'Utilisez Ctrl+Z et Ctrl+Y pour annuler ou rétablir une modification de la scène.',
		'Dupliquez le plateau courant depuis le menu Édition pour comparer deux préparations.',
		'Les vues Dessus, Face et Gauche sont accessibles avec Ctrl+1, Ctrl+3 et Ctrl+5.',
		'Enregistrez le projet avant de lancer un tranchage afin de conserver toute la scène.'
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
			aria-label="Conseil du jour"
			data-testid="tip-of-the-day"
			tabindex="-1"
			use:dialogFocus
		>
			<h2 class="ws-dialog-title mb-4 text-primary">Conseil du jour</h2>
			<p class="min-h-16 text-sm leading-6 text-content-muted" data-testid="tip-text">
				{tips[index]}
			</p>
			<div class="mt-6 flex justify-end gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-secondary"
					onclick={() => (index = (index + 1) % tips.length)}>Suivant</button
				>
				<button type="button" class="ws-btn ws-btn-primary" onclick={onClose}>Fermer</button>
			</div>
		</div>
	</div>
{/if}
