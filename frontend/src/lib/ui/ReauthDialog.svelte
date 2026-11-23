<script lang="ts">
	// Phase 46 (audit 44 SESS-a) : reconnexion sur place quand la session expire.
	// Les requêtes refusées (401) attendent dans `lib/api/reauth` ; une fois
	// reconnecté, elles sont rejouées (l'enregistrement repart avec les
	// modifications gardées en mémoire). Même compte imposé : rejouer pour un
	// autre compte écrirait dans un projet qui n'est pas le sien.
	import { onMount } from 'svelte';
	import Dialog from './Dialog.svelte';
	import { t } from '$lib/i18n';
	import { loginErrorMessage } from '$lib/api/account';
	import { currentUser, login } from '$lib/api/session';
	import { finishReauth, reauthState, setReauthHost, setReauthUser } from '$lib/api/reauth';

	let password = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);

	onMount(() => {
		setReauthHost(true);
		const unsubscribe = currentUser.subscribe((user) => setReauthUser(user?.email ?? null));
		return () => {
			unsubscribe();
			setReauthHost(false);
		};
	});

	$effect(() => {
		if ($reauthState.open) {
			password = '';
			error = null;
		}
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const email = $reauthState.email;
		if (!email || busy) return;
		busy = true;
		error = null;
		try {
			await login(email, password);
			password = '';
			finishReauth(true);
		} catch (e) {
			error = loginErrorMessage(e);
		} finally {
			busy = false;
		}
	}
</script>

<Dialog
	open={$reauthState.open}
	label={$t('Session expired')}
	title={$t('Session expired')}
	size="sm"
	onclose={() => !busy && finishReauth(false)}
	data-testid="reauth-dialog"
>
	<form id="reauth-form" class="flex flex-col gap-3 text-sm" onsubmit={submit}>
		<p class="text-content-muted">
			{$t(
				'Your session has expired. Sign in again to continue: your unsaved changes are kept and the interrupted action (saving, slicing…) is resumed.'
			)}
		</p>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">{$t('Email||auth')}</span>
			<input
				class="ws-field"
				type="email"
				value={$reauthState.email ?? ''}
				readonly
				autocomplete="username"
				data-testid="reauth-email"
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-content-muted">{$t('Password')}</span>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				class="ws-field"
				type="password"
				bind:value={password}
				required
				autofocus
				autocomplete="current-password"
				data-testid="reauth-password"
			/>
		</label>
		{#if error}<p class="text-sm text-danger" role="alert">{error}</p>{/if}
	</form>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			disabled={busy}
			data-testid="reauth-cancel"
			onclick={() => finishReauth(false)}
		>
			{$t('Cancel')}
		</button>
		<button
			type="submit"
			form="reauth-form"
			class="ws-btn ws-btn-primary"
			disabled={busy || !password}
			data-testid="reauth-submit"
		>
			{busy ? $t('Signing in…') : $t('Sign in')}
		</button>
	{/snippet}
</Dialog>
