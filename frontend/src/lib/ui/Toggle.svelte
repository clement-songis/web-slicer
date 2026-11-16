<script lang="ts">
	// Interrupteur (role="switch") pour les réglages binaires immédiats.
	import { cx } from './styles';

	type Props = {
		checked?: boolean;
		label: string;
		disabled?: boolean;
		class?: string;
		onchange?: (checked: boolean) => void;
		[key: `data-${string}`]: string | undefined;
	};

	let {
		checked = $bindable(false),
		label,
		disabled = false,
		class: extra = '',
		onchange,
		...rest
	}: Props = $props();
</script>

<button
	type="button"
	role="switch"
	aria-checked={checked}
	aria-label={label}
	{disabled}
	class={cx(
		'relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors disabled:opacity-50',
		checked ? 'border-primary bg-primary' : 'border-border-strong bg-surface-sunken',
		extra
	)}
	onclick={() => {
		checked = !checked;
		onchange?.(checked);
	}}
	{...rest}
>
	<span
		class={cx(
			'inline-block h-3.5 w-3.5 rounded-full shadow-sm transition-transform',
			checked ? 'translate-x-4 bg-primary-content' : 'translate-x-0.5 bg-content-subtle'
		)}
	></span>
</button>
