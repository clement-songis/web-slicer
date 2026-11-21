<script lang="ts">
	// Éditeur de G-code personnalisé (Orca EditGCodeDialog) : grande zone
	// d'édition + variables disponibles (spécifiques à la clé, états de
	// tranchage, statistiques, dimensions, horodatages, réglages des presets),
	// recherche, description, insertion au curseur (double-clic ou bouton).
	import Dialog from '$lib/ui/Dialog.svelte';
	import { t } from '$lib/i18n';
	import {
		filterPlaceholders,
		insertAt,
		placeholderGroups,
		type PlaceholderItem
	} from './preset-tools';

	interface Props {
		open: boolean;
		gcodeKey: string;
		title: string;
		value: string;
		onapply: (text: string) => void;
		onclose: () => void;
	}
	let { open, gcodeKey, title, value, onapply, onclose }: Props = $props();

	let text = $state('');
	let query = $state('');
	let selected = $state<PlaceholderItem | null>(null);
	let area = $state<HTMLTextAreaElement | null>(null);
	$effect(() => {
		if (open) {
			text = value;
			query = '';
			selected = null;
		}
	});
	const groups = $derived(placeholderGroups(gcodeKey));
	const shown = $derived(filterPlaceholders(groups, query, (s) => $t(s)));
	// Groupes repliés par défaut (sauf la variable spécifique et pendant une recherche).
	let openGroups = $state<Record<string, boolean>>({});

	function insert(item: PlaceholderItem) {
		const el = area;
		const start = el?.selectionStart ?? text.length;
		const end = el?.selectionEnd ?? text.length;
		const res = insertAt(text, start, end, item.insert);
		text = res.value;
		queueMicrotask(() => {
			el?.focus();
			el?.setSelectionRange(res.cursor, res.cursor);
		});
	}
</script>

<Dialog
	{open}
	label={$t('Edit: {title}', { title })}
	title={$t('Edit: {title}', { title })}
	size="xl"
	{onclose}
	data-testid="gcode-editor"
>
	<div class="grid h-[60vh] grid-cols-[18rem_1fr] gap-3">
		<div class="flex min-h-0 flex-col gap-2">
			<input
				type="search"
				class="ws-field ws-field-sm"
				placeholder={$t('Search a variable…')}
				bind:value={query}
				data-testid="gcode-placeholder-search"
				aria-label={$t('Search a variable')}
			/>
			<div
				class="min-h-0 flex-1 overflow-y-auto rounded border border-border"
				role="tree"
				aria-label={$t('Variables')}
			>
				{#each shown as group, gi (group.title)}
					{@const expanded = query.trim() !== '' || (openGroups[group.title] ?? gi === 0)}
					<div role="treeitem" aria-expanded={expanded} aria-selected="false">
						<button
							type="button"
							class="flex w-full items-center justify-between bg-overlay px-2 py-1 text-left text-xs font-semibold"
							onclick={() => (openGroups = { ...openGroups, [group.title]: !expanded })}
							><span
								>{$t(
									group.title,
									group.titleParams ? { label: $t(group.titleParams.label) } : undefined
								)}</span
							><span class="text-content-subtle tabular-nums">{group.items.length}</span></button
						>
						{#if expanded}
							<ul role="group">
								{#each group.items as item (group.title + item.key)}
									<li>
										<button
											type="button"
											class="block w-full truncate px-3 py-0.5 text-left font-mono text-xs hover:bg-overlay {selected ===
											item
												? 'bg-primary/10 text-primary'
												: ''}"
											title={$t(item.label)}
											data-testid="gcode-placeholder"
											data-key={item.key}
											onclick={() => (selected = item)}
											ondblclick={() => insert(item)}>{item.insert}</button
										>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				{:else}
					<p class="p-3 text-xs text-content-subtle">{$t('No variable.')}</p>
				{/each}
			</div>
			<div
				class="min-h-20 rounded border border-border p-2 text-xs"
				aria-live="polite"
				data-testid="gcode-placeholder-help"
			>
				{#if selected}
					<p class="font-medium text-content">{$t(selected.label)}</p>
					<p class="mt-1 whitespace-pre-line text-content-muted">
						{selected.tooltip ? $t(selected.tooltip) : '—'}
					</p>
					<button
						type="button"
						class="ws-btn ws-btn-sm ws-btn-secondary mt-2"
						data-testid="gcode-insert"
						onclick={() => selected && insert(selected)}
						>{$t('Insert {name}', { name: selected.insert })}</button
					>
				{:else}
					<p class="text-content-subtle">
						{$t('Choose a variable (double-click to insert it at the cursor).')}
					</p>
				{/if}
			</div>
		</div>
		<textarea
			bind:this={area}
			bind:value={text}
			spellcheck="false"
			class="ws-field h-full resize-none font-mono text-xs leading-relaxed"
			aria-label={title}
			data-testid="gcode-editor-text"></textarea>
	</div>
	{#snippet footer()}
		<button type="button" class="ws-btn ws-btn-secondary" onclick={onclose}>{$t('Cancel')}</button>
		<button
			type="button"
			class="ws-btn ws-btn-primary"
			data-testid="gcode-editor-apply"
			onclick={() => {
				onapply(text);
				onclose();
			}}>{$t('Apply')}</button
		>
	{/snippet}
</Dialog>
