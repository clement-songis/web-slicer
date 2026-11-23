<script lang="ts">
	// « Définir le nombre d'instances… » (audit 43 F-a) : dialogue de
	// l'application à la place de window.prompt, comme le GetNumberFromUser
	// d'Orca (Plater::set_number_of_copies) — titre « Copies of the selected
	// object », invite « Number of copies: », 0–1000, défaut = instances actuelles.
	import Dialog from '$lib/ui/Dialog.svelte';
	import Button from '$lib/ui/Button.svelte';
	import Input from '$lib/ui/Input.svelte';
	import { t } from '$lib/i18n';
	import { MAX_INSTANCES, parseInstanceCount } from './instances';

	type Props = {
		open: boolean;
		/** Nombre d'instances actuel (valeur initiale). */
		current: number;
		/** Nom de l'objet (rappel sous le titre). */
		name?: string;
		onconfirm: (count: number) => void;
		onclose: () => void;
	};
	let { open, current, name = '', onconfirm, onclose }: Props = $props();

	let text = $state<string | number | null>('');
	let wasOpen = false;
	$effect(() => {
		if (open && !wasOpen) text = String(current);
		wasOpen = open;
	});
	const value = $derived(parseInstanceCount(String(text ?? '')));

	function submit(e: Event) {
		e.preventDefault();
		if (value === null) return;
		onconfirm(value);
	}
</script>

<Dialog
	{open}
	size="sm"
	label={$t('Copies of the selected object')}
	title={$t('Copies of the selected object')}
	{onclose}
	data-testid="instance-count-dialog"
>
	<form id="instance-count-form" class="flex flex-col gap-2" onsubmit={submit}>
		{#if name}<p class="text-muted text-sm" data-testid="instance-count-object">{name}</p>{/if}
		<label class="flex items-center gap-2 text-sm">
			<span>{$t('Number of copies:')}</span>
			<Input
				class="w-24"
				inputmode="numeric"
				bind:value={text}
				aria-invalid={value === null}
				data-testid="instance-count-input"
			/>
			<span class="text-muted text-xs">0–{MAX_INSTANCES}</span>
		</label>
		{#if value === null}
			<p class="text-xs text-danger" role="alert" data-testid="instance-count-error">
				{$t('Enter a whole number from 0 to {max}.', { max: MAX_INSTANCES })}
			</p>
		{/if}
	</form>
	{#snippet footer()}
		<Button variant="ghost" onclick={onclose} data-testid="instance-count-cancel">
			{$t('Cancel')}
		</Button>
		<Button
			type="submit"
			form="instance-count-form"
			disabled={value === null}
			data-testid="instance-count-ok"
		>
			{$t('OK')}
		</Button>
	{/snippet}
</Dialog>
