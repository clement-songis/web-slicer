<script lang="ts" generics="T extends string">
	// Contrôle segmenté (T094) — sélection unique parmi des options courtes
	// (bascule Préparer/Aperçu, filtres…). `value` est bindable.
	import { segmentClasses } from './styles';

	type Option = { value: T; label: string };

	type Props = {
		options: Option[];
		value: T;
		ariaLabel?: string;
	};

	let { options, value = $bindable(), ariaLabel }: Props = $props();
</script>

<div
	class="inline-flex overflow-hidden rounded-md border border-border bg-surface-raised"
	role="group"
	aria-label={ariaLabel}
>
	{#each options as opt (opt.value)}
		<button
			type="button"
			onclick={() => (value = opt.value)}
			aria-pressed={value === opt.value}
			class={segmentClasses(value === opt.value)}
		>
			{opt.label}
		</button>
	{/each}
</div>
