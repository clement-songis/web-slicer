<script lang="ts">
	// Changement de preset avec des modifications non enregistrées (Orca
	// UnsavedChangesDialog) : transférer au nouveau preset, abandonner, ou
	// rester sur le preset actuel. Liste des options modifiées.
	import Dialog from '#lib/ui/Dialog.svelte';
	import { t } from '#lib/i18n';
	import { formatSetting, type SettingDiff } from './preset-tools';

	interface Props {
		open: boolean;
		kindLabel: string;
		fromName: string;
		toName: string;
		changes: SettingDiff[];
		ontransfer: () => void;
		ondiscard: () => void;
		oncancel: () => void;
	}
	let { open, kindLabel, fromName, toName, changes, ontransfer, ondiscard, oncancel }: Props =
		$props();
	// Message traduit découpé autour des noms (affichés en gras).
	const parts = $derived(
		$t(
			'The {kind} preset “{from}” has {n} unsaved modified option(s). What to do when switching to “{to}”?',
			{ kind: $t(kindLabel), n: changes.length }
		).split(/(\{from\}|\{to\})/)
	);
</script>

<Dialog
	{open}
	label={$t('Unsaved changes')}
	title={$t('Unsaved changes')}
	size="lg"
	onclose={oncancel}
	data-testid="unsaved-preset-dialog"
>
	<div class="flex flex-col gap-3 text-sm">
		<p>
			{#each parts as part, i (i)}{#if part === '{from}'}<strong>
						{fromName}
					</strong>{:else if part === '{to}'}<strong>{toName}</strong>{:else}{part}{/if}{/each}
		</p>
		<div class="max-h-64 overflow-y-auto rounded border border-border">
			<table class="w-full text-xs">
				<thead class="sticky top-0 bg-surface-raised text-left text-content-muted">
					<tr>
						<th class="px-3 py-1">{$t('Option')}</th><th class="px-3 py-1">{$t('Saved')}</th><th
							class="px-3 py-1"
						>
							{$t('Modified')}
						</th>
					</tr>
				</thead>
				<tbody>
					{#each changes as c (c.key)}
						<tr class="border-t border-border" data-testid="unsaved-row" data-key={c.key}>
							<td class="px-3 py-1 text-content-muted">{$t(c.label)}</td>
							<td class="px-3 py-1 font-mono">{formatSetting(c.left)}</td>
							<td class="px-3 py-1 font-mono text-warning-content">{formatSetting(c.right)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
	{#snippet footer()}
		<button
			type="button"
			class="ws-btn ws-btn-ghost"
			data-testid="unsaved-cancel"
			onclick={oncancel}
		>
			{$t('Keep “{name}”', { name: fromName })}
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-secondary"
			data-testid="unsaved-discard"
			onclick={ondiscard}
		>
			{$t('Discard changes')}
		</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="unsaved-transfer"
			onclick={ontransfer}
		>
			{$t('Transfer to “{name}”', { name: toName })}
		</button>
	{/snippet}
</Dialog>
