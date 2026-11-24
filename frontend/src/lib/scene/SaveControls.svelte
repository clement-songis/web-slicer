<script lang="ts">
	// Contrôles de sauvegarde (T060) : bouton de sauvegarde manuelle (FR-052),
	// raccourci Ctrl/Cmd+S, et bannière d'avertissement en cas de conflit
	// multi-onglets (verrou optimiste, 409). Le parent fournit `save`, qui
	// construit le document scène et appelle `saveScene`.
	//
	// Ce composant est l'UNIQUE propriétaire de Ctrl/Cmd+S (le dispatcher
	// d'éditeur `editor/shortcuts.ts` ne le mappe pas). Ctrl+Shift+S n'est pas
	// capté ici : il reste au dispatcher (`project.saveAs`).
	import type { SaveOutcome } from './save';
	import { isSaveChord } from '../editor/shortcuts';
	import { t } from '#lib/i18n';

	interface Props {
		/** Effectue la sauvegarde et renvoie son issue (le parent bâtit la scène). */
		save: () => Promise<SaveOutcome>;
		/** Modifications non enregistrées depuis la dernière sauvegarde. */
		unsaved?: boolean;
		/** Audit 42 C2 — conflit : recharger le projet enregistré ailleurs. */
		onreload?: () => void;
		/** Audit 42 C2 — conflit : écraser la version enregistrée ailleurs. */
		overwrite?: () => Promise<SaveOutcome>;
	}

	let { save, unsaved = false, onreload, overwrite }: Props = $props();

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
	async function runOverwrite() {
		if (!overwrite || saveState === 'saving') return;
		saveState = 'saving';
		const outcome = await overwrite();
		saveState = outcome.status === 'saved' ? 'saved' : outcome.status;
		message = outcome.status === 'error' ? outcome.message : '';
	}

	function onKeydown(e: KeyboardEvent) {
		if (
			!isSaveChord({
				key: e.key,
				ctrl: e.ctrlKey,
				shift: e.shiftKey,
				alt: e.altKey,
				meta: e.metaKey
			})
		)
			return;
		// Toujours bloquer « Enregistrer la page » du navigateur, y compris en
		// auto-répétition (touche maintenue) qui, elle, ne relance pas de save.
		e.preventDefault();
		if (e.repeat) return;
		void run();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="flex flex-col gap-2 text-sm">
	<button
		type="button"
		data-testid="save-project"
		class="ws-btn ws-btn-secondary relative self-start"
		disabled={saveState === 'saving'}
		onclick={run}
		data-unsaved={unsaved ? 'true' : 'false'}
		data-tip={unsaved ? $t('Unsaved changes (Ctrl+S)') : $t('Project saved')}
		data-tip-side="bottom"
		aria-describedby={unsaved ? 'save-unsaved-hint' : undefined}
	>
		{#if unsaved}
			<!-- Marqueur discret « non enregistré » (pastille, comme l'astérisque du
			     titre d'Orca) ; texte pour les lecteurs d'écran via aria-describedby. -->
			<span
				class="absolute -top-1 -right-1 h-2.5 w-2.5 rounded-full border-2 border-surface-raised bg-warning"
				data-testid="unsaved-indicator"
				aria-hidden="true"
			></span>
		{/if}
		{saveState === 'saving' ? $t('Saving…||project') : $t('Save||project')}
	</button>
	{#if unsaved}
		<span id="save-unsaved-hint" class="sr-only">{$t('Unsaved changes')}</span>
	{/if}

	<!-- Succès : annoncé par la barre de message de l'éditeur (évite le doublon
	     et le décalage de la barre d'en-tête). -->
	{#if saveState === 'conflict'}
		<!-- Audit 42 C2 : l'autre écriture peut venir d'un autre onglet ou d'un autre
		     appareil ; deux actions explicites plutôt qu'une consigne. -->
		<div class="flex flex-col gap-1" role="alert" data-testid="save-conflict">
			<p class="text-warning">
				{$t(
					'Conflict: the project was modified elsewhere (another tab or device). Reload to see those changes (yours will be lost), or overwrite them with your version.'
				)}
			</p>
			<div class="flex gap-2">
				{#if onreload}<button
						type="button"
						class="ws-btn ws-btn-secondary ws-btn-sm"
						data-testid="save-conflict-reload"
						onclick={() => onreload?.()}
					>
						{$t('Reload')}
					</button>{/if}
				{#if overwrite}<button
						type="button"
						class="ws-btn ws-btn-danger ws-btn-sm"
						data-testid="save-conflict-overwrite"
						onclick={runOverwrite}
					>
						{$t('Overwrite||conflict')}
					</button>{/if}
			</div>
		</div>
	{:else if saveState === 'error'}
		<p class="text-danger" role="alert">{message}</p>
	{/if}
</div>
