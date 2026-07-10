<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ApiError } from '$lib/api/client';
	import { register } from '$lib/api/session';

	let email = $state('');
	let password = $state('');
	let inviteToken = $state('');
	let error = $state<string | null>(null);
	let busy = $state(false);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		busy = true;
		error = null;
		try {
			await register(email, password, inviteToken.trim() || undefined);
			await goto(resolve('/library'));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}
</script>

<h1 class="mb-6 text-xl font-semibold text-gray-900 dark:text-gray-100">Créer un compte</h1>

<form class="space-y-4" onsubmit={submit}>
	<label class="block">
		<span class="mb-1 block text-sm text-gray-700 dark:text-gray-300">Email</span>
		<input
			type="email"
			bind:value={email}
			required
			autocomplete="email"
			class="w-full rounded border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
		/>
	</label>

	<label class="block">
		<span class="mb-1 block text-sm text-gray-700 dark:text-gray-300">
			Mot de passe <span class="text-gray-400">(8 caractères min.)</span>
		</span>
		<input
			type="password"
			bind:value={password}
			required
			minlength={8}
			autocomplete="new-password"
			class="w-full rounded border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
		/>
	</label>

	<label class="block">
		<span class="mb-1 block text-sm text-gray-700 dark:text-gray-300">
			Jeton d'invitation <span class="text-gray-400">(si requis)</span>
		</span>
		<input
			type="text"
			bind:value={inviteToken}
			autocomplete="off"
			class="w-full rounded border border-gray-300 px-3 py-2 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
		/>
	</label>

	{#if error}
		<p class="text-sm text-red-600" role="alert">{error}</p>
	{/if}

	<button
		type="submit"
		disabled={busy}
		class="w-full rounded bg-blue-600 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-50"
	>
		{busy ? 'Création…' : 'Créer le compte'}
	</button>
</form>

<p class="mt-4 text-center text-sm text-gray-600 dark:text-gray-400">
	Déjà inscrit ? <a href={resolve('/login')} class="text-blue-600 hover:underline">Se connecter</a>
</p>
