<script lang="ts">
	// Bascule de thème (T093) : contrôle segmenté clair / sombre / système.
	// Présentationnel — l'état et la persistance vivent dans `lib/theme`.
	import { t } from '$lib/i18n';
	import { themePref, setTheme, type ThemePref } from './theme';

	const options: { value: ThemePref; label: string; icon: string }[] = [
		{ value: 'light', label: 'Light', icon: '☀' },
		{ value: 'dark', label: 'Dark', icon: '☾' },
		{ value: 'system', label: 'System', icon: '⌂' }
	];
</script>

<div
	class="inline-flex overflow-hidden rounded-md border border-border bg-surface-raised"
	role="group"
	aria-label={$t('Theme')}
>
	{#each options as opt (opt.value)}
		<button
			type="button"
			onclick={() => setTheme(opt.value)}
			aria-pressed={$themePref === opt.value}
			title={$t(opt.label)}
			class="px-2 py-1 text-sm transition-colors {$themePref === opt.value
				? 'bg-primary text-primary-content'
				: 'text-content-muted hover:bg-overlay'}"
		>
			<span aria-hidden="true">{opt.icon}</span>
			<span class="sr-only">{$t(opt.label)}</span>
		</button>
	{/each}
</div>
