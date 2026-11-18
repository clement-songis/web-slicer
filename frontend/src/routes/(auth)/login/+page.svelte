<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { login } from '$lib/api/session';

	let email = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);
	let hydrated = $state(false);

	// Évite une soumission HTML native si l'utilisateur clique pendant le
	// chargement des modules, avant que le gestionnaire Svelte soit branché.
	onMount(() => {
		hydrated = true;
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = null;
		try {
			await login(email, password);
			// Racine : page de démarrage choisie (préférence Orca default_page).
			await goto(resolve('/'));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Connexion — Web-Slicer</title></svelte:head>

<h1 class="ws-page-title mb-6">Connexion</h1>

<form class="space-y-4" onsubmit={submit}>
	<label class="block">
		<span class="mb-1 block text-sm text-content-muted">Email</span>
		<input
			type="email"
			bind:value={email}
			required
			autocomplete="email"
			data-testid="auth-email"
			class="ws-field ws-field-lg w-full"
		/>
	</label>

	<label class="block">
		<span class="mb-1 block text-sm text-content-muted">Mot de passe</span>
		<input
			type="password"
			bind:value={password}
			required
			autocomplete="current-password"
			data-testid="auth-password"
			class="ws-field ws-field-lg w-full"
		/>
	</label>

	{#if error}
		<p class="text-sm text-danger" role="alert">{error}</p>
	{/if}

	<button
		type="submit"
		data-testid="auth-submit"
		disabled={!hydrated || busy}
		class="ws-btn ws-btn-lg ws-btn-primary w-full"
	>
		{busy ? 'Connexion…' : 'Se connecter'}
	</button>
</form>

<p class="mt-4 text-center text-sm text-content-muted">
	Pas de compte ?
	<a href={resolve('/register')} class="text-primary hover:underline">Créer un compte</a>
</p>
