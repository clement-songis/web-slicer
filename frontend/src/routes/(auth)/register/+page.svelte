<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { register } from '$lib/api/session';

	let email = $state('');
	let password = $state('');
	let inviteToken = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);
	let hydrated = $state(false);

	// Sans JS, le bouton reste inactif plutôt que de recharger la page en GET
	// avec les champs du formulaire dans l'URL avant l'hydratation.
	onMount(() => {
		hydrated = true;
		// Lien d'invitation (page Administration) : /register?invite=<jeton>.
		const invite = new URLSearchParams(location.search).get('invite');
		if (invite && !inviteToken) inviteToken = invite;
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		error = null;
		// Validation en français dans le formulaire (la bulle native suit la langue
		// du navigateur et disparaît aussitôt).
		if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())) {
			error = 'Adresse e-mail invalide.';
			return;
		}
		if (password.length < 8) {
			error = `Mot de passe trop court : ${password.length} caractère(s), 8 minimum.`;
			return;
		}
		busy = true;
		try {
			await register(email, password, inviteToken.trim() || undefined);
			// Première connexion → wizard de sélection d'imprimante (Phase 14).
			await goto(resolve('/setup'));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>Créer un compte — Web-Slicer</title></svelte:head>

<h1 class="ws-page-title mb-6">Créer un compte</h1>

<form class="space-y-4" onsubmit={submit} novalidate>
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
		<span class="mb-1 block text-sm text-content-muted">
			Mot de passe <span class="text-content-subtle">(8 caractères min.)</span>
		</span>
		<input
			type="password"
			bind:value={password}
			required
			minlength={8}
			autocomplete="new-password"
			data-testid="auth-password"
			class="ws-field ws-field-lg w-full"
		/>
	</label>

	<label class="block">
		<span class="mb-1 block text-sm text-content-muted">
			Jeton d'invitation <span class="text-content-subtle">(si requis)</span>
		</span>
		<input
			type="text"
			bind:value={inviteToken}
			data-testid="auth-invite"
			autocomplete="off"
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
		{busy ? 'Création…' : 'Créer le compte'}
	</button>
</form>

<p class="mt-4 text-center text-sm text-content-muted">
	Déjà inscrit ? <a href={resolve('/login')} class="text-primary hover:underline">Se connecter</a>
</p>
