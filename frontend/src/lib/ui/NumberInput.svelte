<script lang="ts">
	// Champ numérique avec unité optionnelle (mm, °C, %) alignée à droite.
	// Chiffres tabulaires, valeur alignée à droite (lecture en colonne).
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cx, fieldClasses, type ButtonSizeFull } from './styles';

	type Props = Omit<HTMLInputAttributes, 'size' | 'type'> & {
		value?: number | null;
		unit?: string;
		size?: ButtonSizeFull;
		class?: string;
	};

	let { value = $bindable(), unit, size = 'md', class: extra = '', ...rest }: Props = $props();
</script>

<span class={cx('relative inline-flex items-center', extra)}>
	<input
		type="number"
		class={cx(fieldClasses(size), 'w-full text-right', unit && 'pr-8')}
		bind:value
		{...rest}
	/>
	{#if unit}
		<span
			class="pointer-events-none absolute right-2 text-xs text-content-subtle"
			aria-hidden="true">{unit}</span
		>
	{/if}
</span>
