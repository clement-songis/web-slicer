<script lang="ts">
	// Pastille d'état (compte, statut). Tonalités sémantiques uniquement.
	import type { Snippet } from 'svelte';
	import { cx } from './styles';

	type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger';
	type Props = { tone?: Tone; class?: string; children: Snippet };

	let { tone = 'neutral', class: extra = '', children }: Props = $props();

	const TONE: Record<Tone, string> = {
		neutral: '',
		accent: 'border-primary/40 text-primary',
		success: 'border-transparent bg-success-soft text-success-content',
		warning: 'border-transparent bg-warning-soft text-warning-content',
		danger: 'border-transparent bg-danger-soft text-danger-content'
	};
</script>

<span class={cx('ws-badge', TONE[tone], extra)}>{@render children()}</span>
