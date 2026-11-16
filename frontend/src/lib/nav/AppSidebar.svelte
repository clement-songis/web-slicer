<script lang="ts">
	// Barre latérale de navigation partagée (parité orca-home.png) : compte +
	// liens Récent / Imprimantes / File + thème + déconnexion. Montée par la
	// bibliothèque, la page imprimantes et la file pour garder le menu partout.
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { logout } from '$lib/api/session';
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
		active?: 'recent' | 'printers' | 'queue';
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

	const ITEM = 'flex items-center gap-2 rounded-md px-3 py-2 transition-colors';
	const IDLE = `${ITEM} text-content-muted hover:bg-overlay hover:text-content`;
	const ACTIVE = `${ITEM} bg-primary/10 font-medium text-primary`;
</script>

<aside class="flex w-64 shrink-0 flex-col border-r border-border bg-surface-raised">
	<details class="relative border-b border-border p-3" data-testid="user-menu">
		<summary
			class="flex cursor-pointer list-none items-center gap-3 rounded p-2 hover:bg-overlay"
			aria-label="Menu utilisateur"
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
			<button onclick={signOut} disabled={busy} class="ws-menu-item rounded-md py-2">
				{busy ? 'Déconnexion…' : 'Déconnexion'}
			</button>
			{#if error}
				<p class="px-3 py-2 text-xs text-danger" role="alert">{error}</p>
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
