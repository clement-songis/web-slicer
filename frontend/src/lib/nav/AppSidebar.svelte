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

	interface Props {
		/** Adresse e-mail du compte connecté (avatar + libellé). */
		email: string;
		/** Entrée active (surlignée). */
		active: 'recent' | 'printers' | 'queue';
	}

	let { email, active }: Props = $props();

	let busy = $state(false);

	async function signOut() {
		if (busy) return;
		busy = true;
		try {
			await logout();
			await goto(resolve('/login'));
		} finally {
			busy = false;
		}
	}

	const ITEM = 'flex items-center gap-2 rounded px-3 py-2';
	const IDLE = `${ITEM} text-content-muted hover:bg-overlay`;
	const ACTIVE = `${ITEM} bg-primary/10 font-medium text-primary`;
</script>

<aside class="flex w-64 shrink-0 flex-col border-r border-border bg-surface-raised">
	<div class="flex flex-col items-center gap-2 border-b border-border px-4 py-6">
		<span class="text-sm font-semibold text-content">Mon compte</span>
		<div
			class="flex h-16 w-16 items-center justify-center rounded-xl bg-overlay text-2xl font-semibold text-content-muted"
			aria-hidden="true"
		>
			{email.charAt(0).toUpperCase()}
		</div>
		<span class="max-w-full truncate text-xs text-content-muted" title={email}>
			{email}
		</span>
		<button onclick={signOut} disabled={busy} class="text-xs text-primary hover:underline">
			Déconnexion
		</button>
	</div>
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
			<IconQueue class="h-4 w-4" /> File
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
