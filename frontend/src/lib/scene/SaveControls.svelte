<script lang="ts">
	// Contrôles de sauvegarde (T060) : bouton de sauvegarde manuelle (FR-052),
	// raccourci Ctrl/Cmd+S, et bannière d'avertissement en cas de conflit
	// multi-onglets (verrou optimiste, 409). Le parent fournit `save`, qui
	// construit le document scène et appelle `saveScene`.
	import { onSaveShortcut, type SaveOutcome } from './save';

	interface Props {
		/** Effectue la sauvegarde et renvoie son issue (le parent bâtit la scène). */
		save: () => Promise<SaveOutcome>;
	}

	let { save }: Props = $props();

	type SaveState = 'idle' | 'saving' | 'saved' | 'conflict' | 'error';
	let saveState = $state<SaveState>('idle');
	let message = $state('');

	async function run() {
		if (saveState === 'saving') return;
		saveState = 'saving';
		const outcome = await save();
		saveState = outcome.status === 'saved' ? 'saved' : outcome.status;
		message = outcome.status === 'error' ? outcome.message : '';
	}

	$effect(() => onSaveShortcut(run));
</script>

<div class="flex flex-col gap-2 text-sm">
	<button
		type="button"
		class="self-start rounded bg-sky-600 px-3 py-1 text-white hover:bg-sky-500 disabled:opacity-50"
		disabled={saveState === 'saving'}
		onclick={run}
	>
		{saveState === 'saving' ? 'Sauvegarde…' : 'Sauvegarder'}
	</button>

	{#if saveState === 'saved'}
		<p class="text-success">Projet sauvegardé.</p>
	{:else if saveState === 'conflict'}
		<p class="text-warning" role="alert">
			Conflit : le projet a été modifié dans un autre onglet. Rechargez avant de sauvegarder pour ne
			pas écraser ces changements.
		</p>
	{:else if saveState === 'error'}
		<p class="text-danger" role="alert">{message}</p>
	{/if}
</div>
