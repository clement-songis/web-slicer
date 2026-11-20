<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { loginErrorMessage } from '$lib/api/account';
	import { login } from '$lib/api/session';
	import { safeNext } from '$lib/api/return-to';

	let email = $state('');
	let password = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);
	let hydrated = $state(false);
	let showForgot = $state(false);

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
			// Page demandée avant la connexion (?next=), sinon racine : page de
			// démarrage choisie (préférence Orca default_page).
			const next = safeNext(new URLSearchParams(location.search).get('next'));
			// eslint-disable-next-line svelte/no-navigation-without-resolve -- chemin interne validé par safeNext
			await goto(next ?? resolve('/'));
		} catch (err) {
			// Lot 32 : 401 `invalid_credentials` (même réponse si l'e-mail est
			// inconnu) ; repli 401 générique des serveurs antérieurs.
			error = loginErrorMessage(err);
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

	<div class="-mt-2 text-right">
		<button
			type="button"
			class="text-xs text-primary hover:underline"
			aria-expanded={showForgot}
			aria-controls="forgot-help"
			data-testid="auth-forgot"
			onclick={() => (showForgot = !showForgot)}
		>
			Mot de passe oublié ?
		</button>
	</div>

	{#if showForgot}
		<p
			id="forgot-help"
			class="rounded border border-border bg-overlay px-3 py-2 text-xs text-content-muted"
			data-testid="auth-forgot-help"
		>
			Cette instance n'envoie pas d'e-mails : demandez à son administrateur de réinitialiser votre
			mot de passe (page Administration). Vous pourrez ensuite vous connecter avec le mot de passe
			qu'il vous communiquera.
		</p>
	{/if}

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
