<script lang="ts">
	// Champ de formulaire (T094) — libellé + contrôle (snippet) + message d'erreur.
	// Le contrôle réel (input/select) est fourni par l'appelant en enfant afin de
	// garder le `bind:value` et la logique côté page.
	import type { Snippet } from 'svelte';
	import { cx } from './styles';

	type Props = {
		label: string;
		error?: string | null;
		for?: string;
		class?: string;
		children: Snippet;
	};

	let { label, error = null, for: htmlFor, class: extra = '', children }: Props = $props();
</script>

<div class={cx('flex flex-col gap-1', extra)}>
	<label class="text-sm font-medium text-content-muted" for={htmlFor}>{label}</label>
	{@render children()}
	{#if error}
		<p class="text-sm text-danger" role="alert">{error}</p>
	{/if}
</div>
