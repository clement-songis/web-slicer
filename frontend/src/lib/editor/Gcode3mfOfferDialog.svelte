<script lang="ts">
	// Ouverture d'un projet refusée par le serveur parce que le fichier est un
	// fichier tranché .gcode.3mf (lot 44 : 422 `details.kind = "gcode_3mf"`) :
	// proposer de l'ouvrir dans l'aperçu G-code, comme Orca qui rouvre un
	// .gcode.3mf en aperçu (`Plater::is_gcode_3mf`).
	import Dialog from '$lib/ui/Dialog.svelte';
	import Button from '$lib/ui/Button.svelte';
	import { t } from '$lib/i18n';

	type Props = {
		open: boolean;
		filename: string;
		/** Plateaux tranchés de l'archive (détails du 422). */
		plates: readonly number[];
		onopen: () => void;
		onclose: () => void;
	};
	let { open, filename, plates, onopen, onclose }: Props = $props();
</script>

<Dialog
	{open}
	size="sm"
	label={$t('Sliced file')}
	title={$t('Sliced file')}
	{onclose}
	data-testid="gcode3mf-offer"
>
	<p class="text-sm" data-testid="gcode3mf-offer-text">
		{$t(
			'“{file}” is a sliced file (.gcode.3mf): it contains the G-code of {n} plate(s) but no model to import. Open it in the G-code preview?',
			{ file: filename, n: plates.length || 1 }
		)}
	</p>
	{#snippet footer()}
		<Button variant="ghost" onclick={onclose} data-testid="gcode3mf-offer-cancel">
			{$t('Cancel')}
		</Button>
		<Button onclick={onopen} data-testid="gcode3mf-offer-open">
			{$t('Open in G-code preview')}
		</Button>
	{/snippet}
</Dialog>
