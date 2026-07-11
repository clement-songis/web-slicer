<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { login } from '$lib/api/session';

	let email = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = null;
		try {
			await login(email, password);
			await goto(resolve('/library'));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<h1 class="mb-6 text-xl font-semibold text-content">Connexion</h1>

<form class="space-y-4" onsubmit={submit}>
	<label class="block">
		<span class="mb-1 block text-sm text-content-muted">Email</span>
		<input
			type="email"
			bind:value={email}
			required
			autocomplete="email"
			class="w-full rounded border border-border-strong px-3 py-2 bg-surface-raised text-content"
		/>
	</label>

	<label class="block">
		<span class="mb-1 block text-sm text-content-muted">Mot de passe</span>
		<input
			type="password"
			bind:value={password}
			required
			autocomplete="current-password"
			class="w-full rounded border border-border-strong px-3 py-2 bg-surface-raised text-content"
		/>
	</label>

	{#if error}
		<p class="text-sm text-danger" role="alert">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={busy}
		class="w-full rounded bg-primary py-2 font-medium text-white hover:bg-primary-hover disabled:opacity-50"
	>
		{busy ? 'Connexion…' : 'Se connecter'}
	</button>
</form>

<p class="mt-4 text-center text-sm text-content-muted">
	Pas de compte ?
	<a href={resolve('/register')} class="text-primary hover:underline">Créer un compte</a>
</p>
