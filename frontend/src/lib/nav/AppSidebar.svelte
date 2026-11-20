<script lang="ts">
	// Barre latérale de navigation partagée (parité orca-home.png) : compte +
	// liens Récent / Imprimantes / File + thème + déconnexion. Montée par la
	// bibliothèque, la page imprimantes et la file pour garder le menu partout.
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { currentUser, logout } from '$lib/api/session';
	import { deleteAccount } from '$lib/api/auth';
	import {
		changePassword,
		getUsage,
		passwordChangeError,
		validatePasswordChange
	} from '$lib/api/account';
	import { gauge, usageLines } from '$lib/api/quota';
	import type { UsageResponse } from '$lib/api/types';
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import IconShield from '~icons/lucide/shield';
	import { ThemeToggle } from '$lib/theme';
	import IconClock from '~icons/lucide/clock';
	import IconPrinter from '~icons/lucide/printer';
	import IconQueue from '~icons/lucide/list-checks';
	import IconExternal from '~icons/lucide/external-link';
	import IconChevronDown from '~icons/lucide/chevron-down';

	interface Props {
		/** Adresse e-mail du compte connecté (avatar + libellé). */
		email: string;
		/** Entrée active (surlignée), absente pendant l'onboarding. */
		active?: 'recent' | 'printers' | 'queue' | 'admin';
	}

	let { email, active }: Props = $props();

	let busy = $state(false);
	let error = $state<string | null>(null);

	async function signOut() {
		if (busy) return;
		busy = true;
		error = null;
		try {
			await logout();
			await goto(resolve('/login'));
		} catch (err) {
			error = err instanceof Error ? err.message : 'Erreur inattendue';
		} finally {
			busy = false;
		}
	}

	// Suppression du compte (DELETE /api/auth/me, mot de passe exigé) — phase 32 :
	// l'API existait sans interface.
	let deleting = $state(false);
	let deletePassword = $state('');
	let deleteError = $state<string | null>(null);
	async function removeAccount(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		deleteError = null;
		try {
			await deleteAccount(deletePassword);
			currentUser.set(null);
			await goto(resolve('/register'));
		} catch (err) {
			deleteError =
				err instanceof ApiError && err.status === 401
					? 'Mot de passe incorrect.'
					: err instanceof Error
						? err.message
						: 'Suppression impossible';
		} finally {
			busy = false;
		}
	}

	// Lot 32 : usage et quotas du compte (GET /me/usage) — jauge du menu
	// compte ; masquée si le serveur ne l'expose pas (404).
	let usage = $state<UsageResponse | null>(null);
	const storageGauge = $derived(
		usage ? gauge(usage.storage_bytes, usage.storage_quota_bytes) : null
	);
	const lines = $derived(usage ? usageLines(usage) : null);
	function refreshUsage() {
		getUsage()
			.then((u) => (usage = u))
			.catch(() => (usage = null));
	}
	onMount(refreshUsage);

	// « Changer mon mot de passe » (PUT /me/password, lot 32) : les autres
	// sessions sont révoquées par le serveur, celle-ci reste ouverte.
	let changing = $state(false);
	let currentPassword = $state('');
	let nextPassword = $state('');
	let confirmPassword = $state('');
	let passwordMsg = $state<{ ok: boolean; text: string } | null>(null);
	async function submitPassword(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const invalid = validatePasswordChange(currentPassword, nextPassword, confirmPassword);
		if (invalid) {
			passwordMsg = { ok: false, text: invalid };
			return;
		}
		busy = true;
		passwordMsg = null;
		try {
			await changePassword({ current_password: currentPassword, new_password: nextPassword });
			passwordMsg = {
				ok: true,
				text: 'Mot de passe changé. Vos autres sessions ont été déconnectées.'
			};
			currentPassword = nextPassword = confirmPassword = '';
			changing = false;
		} catch (err) {
			passwordMsg = { ok: false, text: passwordChangeError(err) };
		} finally {
			busy = false;
		}
	}
	function cancelPassword() {
		changing = false;
		currentPassword = nextPassword = confirmPassword = '';
		passwordMsg = null;
	}

	const ITEM = 'flex items-center gap-2 rounded-md px-3 py-2 transition-colors';
	const IDLE = `${ITEM} text-content-muted hover:bg-overlay hover:text-content`;
	const ACTIVE = `${ITEM} bg-primary/10 font-medium text-primary`;
</script>

<aside class="flex w-64 shrink-0 flex-col border-r border-border bg-surface-raised">
	<details class="relative border-b border-border p-3" data-testid="user-menu">
		<summary
			class="flex cursor-pointer list-none items-center gap-3 rounded p-2 hover:bg-overlay"
			aria-label="Menu utilisateur"
			onclick={refreshUsage}
		>
			<span
				class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-overlay text-lg font-semibold text-content-muted"
				aria-hidden="true"
			>
				{email.charAt(0).toUpperCase()}
			</span>
			<span class="min-w-0 flex-1">
				<span class="block text-sm font-semibold text-content">Mon compte</span>
				<span class="block truncate text-xs text-content-muted" title={email}>{email}</span>
			</span>
			<IconChevronDown class="h-4 w-4 shrink-0 text-content-muted" aria-hidden="true" />
		</summary>
		<div
			class="absolute left-3 right-3 top-full z-20 rounded border border-border bg-surface-raised p-2 shadow-lg"
		>
			{#if usage && lines && storageGauge}
				<div class="flex flex-col gap-1 px-3 py-2 text-xs" data-testid="account-usage">
					<div class="flex justify-between gap-2">
						<span class="text-content-muted">Stockage</span>
						<span class="tabular-nums text-content" data-testid="account-usage-storage"
							>{lines.storage}</span
						>
					</div>
					{#if storageGauge.percent !== null}
						<div
							class="h-1.5 overflow-hidden rounded-full bg-overlay"
							role="meter"
							aria-label="Stockage utilisé"
							aria-valuemin="0"
							aria-valuemax="100"
							aria-valuenow={Math.round(storageGauge.percent)}
						>
							<div
								class="h-full rounded-full {storageGauge.level === 'full'
									? 'bg-danger'
									: storageGauge.level === 'warn'
										? 'bg-warning'
										: 'bg-primary'}"
								style:width="{storageGauge.percent}%"
							></div>
						</div>
					{/if}
					<div class="flex justify-between gap-2">
						<span class="text-content-muted">Tranchages</span>
						<span class="text-right tabular-nums text-content" data-testid="account-usage-slices"
							>{lines.slices}</span
						>
					</div>
				</div>
			{/if}
			{#if !changing}
				<button
					type="button"
					class="ws-menu-item rounded-md py-2"
					onclick={() => {
						changing = true;
						passwordMsg = null;
					}}
					data-testid="account-password-open">Changer mon mot de passe…</button
				>
			{:else}
				<form
					class="flex flex-col gap-2 px-2 py-2 text-xs"
					onsubmit={submitPassword}
					data-testid="account-password-form"
					novalidate
				>
					<!-- Libellés visibles (le menu est étroit : un texte indicatif serait
					     tronqué et disparaîtrait à la saisie). -->
					<label class="flex flex-col gap-0.5 text-content-muted">
						Mot de passe actuel
						<input
							type="password"
							class="ws-field ws-field-sm w-full"
							aria-label="Mot de passe actuel"
							autocomplete="current-password"
							bind:value={currentPassword}
						/>
					</label>
					<label class="flex flex-col gap-0.5 text-content-muted">
						<span
							>Nouveau mot de passe <span class="text-content-subtle">(8 caractères min.)</span
							></span
						>
						<input
							type="password"
							class="ws-field ws-field-sm w-full"
							aria-label="Nouveau mot de passe"
							autocomplete="new-password"
							bind:value={nextPassword}
						/>
					</label>
					<label class="flex flex-col gap-0.5 text-content-muted">
						Confirmer le nouveau mot de passe
						<input
							type="password"
							class="ws-field ws-field-sm w-full"
							aria-label="Confirmer le nouveau mot de passe"
							autocomplete="new-password"
							bind:value={confirmPassword}
						/>
					</label>
					<div class="flex gap-1">
						<button type="submit" class="ws-btn ws-btn-sm ws-btn-primary flex-1" disabled={busy}
							>Changer</button
						>
						<button type="button" class="ws-btn ws-btn-sm ws-btn-secondary" onclick={cancelPassword}
							>Annuler</button
						>
					</div>
				</form>
			{/if}
			{#if passwordMsg}
				<p
					class="px-3 py-1 text-xs {passwordMsg.ok ? 'text-success' : 'text-danger'}"
					role={passwordMsg.ok ? 'status' : 'alert'}
					data-testid="account-password-message"
				>
					{passwordMsg.text}
				</p>
			{/if}
			<button onclick={signOut} disabled={busy} class="ws-menu-item rounded-md py-2">
				{busy ? 'Déconnexion…' : 'Déconnexion'}
			</button>
			{#if error}
				<p class="px-3 py-2 text-xs text-danger" role="alert">{error}</p>
			{/if}
			{#if !deleting}
				<button
					type="button"
					class="ws-menu-item rounded-md py-2 text-danger"
					onclick={() => (deleting = true)}
					data-testid="account-delete-open"
				>
					Supprimer mon compte…
				</button>
			{:else}
				<form class="flex flex-col gap-2 px-2 py-2 text-xs" onsubmit={removeAccount}>
					<p class="text-content-muted">
						Supprime définitivement le compte, ses projets, modèles et G-code.
					</p>
					<input
						type="password"
						class="ws-field ws-field-sm w-full"
						placeholder="Mot de passe"
						aria-label="Mot de passe pour confirmer"
						autocomplete="current-password"
						bind:value={deletePassword}
						required
					/>
					{#if deleteError}<p class="text-danger" role="alert">{deleteError}</p>{/if}
					<div class="flex gap-1">
						<button
							type="submit"
							class="ws-btn ws-btn-sm ws-btn-danger flex-1"
							disabled={busy || !deletePassword}>Supprimer</button
						>
						<button
							type="button"
							class="ws-btn ws-btn-sm ws-btn-secondary"
							onclick={() => {
								deleting = false;
								deletePassword = '';
								deleteError = null;
							}}>Annuler</button
						>
					</div>
				</form>
			{/if}
		</div>
	</details>
	<nav class="flex flex-col gap-1 p-2 text-sm">
		<a
			href={resolve('/library')}
			class={active === 'recent' ? ACTIVE : IDLE}
			aria-current={active === 'recent' ? 'page' : undefined}
		>
			<IconClock class="h-4 w-4" /> Récent
		</a>
		<a
			href={resolve('/printers')}
			class={active === 'printers' ? ACTIVE : IDLE}
			aria-current={active === 'printers' ? 'page' : undefined}
		>
			<IconPrinter class="h-4 w-4" /> Imprimantes
		</a>
		<a
			href={resolve('/queue')}
			class={active === 'queue' ? ACTIVE : IDLE}
			aria-current={active === 'queue' ? 'page' : undefined}
		>
			<IconQueue class="h-4 w-4" /> File de tranchage
		</a>
		{#if $currentUser?.role === 'admin'}
			<a
				href={resolve('/admin')}
				class={active === 'admin' ? ACTIVE : IDLE}
				aria-current={active === 'admin' ? 'page' : undefined}
				data-testid="nav-admin"
			>
				<IconShield class="h-4 w-4" /> Administration
			</a>
		{/if}
		<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
		<a
			href="https://github.com/clement-songis/web-slicer"
			target="_blank"
			rel="noreferrer"
			class="flex items-center justify-between rounded px-3 py-2 text-content-muted hover:bg-overlay"
		>
			<span>GitHub</span>
			<IconExternal class="h-3.5 w-3.5" />
		</a>
	</nav>
	<div class="mt-auto flex items-center justify-between border-t border-border px-3 py-3">
		<span class="text-xs text-content-subtle">Web-Slicer</span>
		<ThemeToggle />
	</div>
</aside>
