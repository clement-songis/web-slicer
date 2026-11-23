<script lang="ts">
	// Bulle d'aide légère : visible au survol ET au focus clavier du
	// déclencheur (WCAG 1.4.13). Le texte est aussi exposé via aria-describedby.
	import type { Snippet } from 'svelte';

	type Props = { text: string; side?: 'top' | 'bottom'; children: Snippet };

	let { text, side = 'top', children }: Props = $props();
	const id = 'tt-' + Math.random().toString(36).slice(2, 9);
</script>

<span class="group/tt relative inline-flex" aria-describedby={id}>
	{@render children()}
	<span
		{id}
		role="tooltip"
		class="ws-tooltip invisible absolute left-1/2 z-50 -translate-x-1/2 whitespace-nowrap opacity-0 transition-opacity group-focus-within/tt:visible group-focus-within/tt:opacity-100 group-hover/tt:visible group-hover/tt:opacity-100 {side ===
		'top'
			? 'bottom-full mb-1.5'
			: 'top-full mt-1.5'}"
	>
		{text}
	</span>
</span>
