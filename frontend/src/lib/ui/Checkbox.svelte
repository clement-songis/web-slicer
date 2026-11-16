<script lang="ts">
	// Case à cocher + libellé cliquable. Couleur via accent-color (layout.css).
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { cx } from './styles';

	type Props = Omit<HTMLInputAttributes, 'type'> & {
		checked?: boolean;
		label: string;
		class?: string;
	};

	let { checked = $bindable(false), label, class: extra = '', ...rest }: Props = $props();
</script>

<label class={cx('inline-flex cursor-pointer items-center gap-2 text-sm text-content', extra)}>
	<input
		type="checkbox"
		class="h-4 w-4 shrink-0 rounded border-border-strong"
		bind:checked
		{...rest}
	/>
	<span>{label}</span>
</label>
