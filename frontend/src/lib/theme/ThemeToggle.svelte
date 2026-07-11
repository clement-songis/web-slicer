<script lang="ts">
	// Bascule de thème (T093) : contrôle segmenté clair / sombre / système.
	// Présentationnel — l'état et la persistance vivent dans `lib/theme`.
	import type { Component } from 'svelte';
	import IconSun from '~icons/lucide/sun';
	import IconMoon from '~icons/lucide/moon';
	import IconMonitor from '~icons/lucide/monitor';
	import { t } from '$lib/i18n';
	import { themePref, setTheme, type ThemePref } from './theme';

	const options: { value: ThemePref; label: string; icon: Component }[] = [
		{ value: 'light', label: 'Light', icon: IconSun },
		{ value: 'dark', label: 'Dark', icon: IconMoon },
		{ value: 'system', label: 'System', icon: IconMonitor }
	];
</script>

<div
	class="inline-flex overflow-hidden rounded-md border border-border bg-surface-raised"
	role="group"
	aria-label={$t('Theme')}
>
	{#each options as opt (opt.value)}
		{@const Icon = opt.icon}
		<button
			type="button"
			onclick={() => setTheme(opt.value)}
			aria-pressed={$themePref === opt.value}
			title={$t(opt.label)}
			class="px-2 py-1 text-sm transition-colors {$themePref === opt.value
				? 'bg-primary text-primary-content'
				: 'text-content-muted hover:bg-overlay'}"
		>
			<Icon class="h-4 w-4" aria-hidden="true" />
			<span class="sr-only">{$t(opt.label)}</span>
		</button>
	{/each}
</div>
