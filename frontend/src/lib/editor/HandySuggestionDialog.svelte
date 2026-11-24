<script lang="ts">
	// Suggestion après l'ajout d'un modèle pratique (Orca : « String Hell »,
	// GUI_Factories.cpp) : réglage du traitement à appliquer, en clair, avec les
	// valeurs avant → après.
	import { dialogFocus } from '#lib/ui/dialog-focus';
	import { locale, t } from '#lib/i18n';
	import { suggestionText, type HandyModelSuggestion } from './handy-models';

	interface Props {
		suggestion: HandyModelSuggestion;
		/** Valeurs actuelles du traitement édité (avant → après). */
		values: Record<string, unknown>;
		onapply: () => void;
		onignore: () => void;
	}
	let { suggestion, values, onapply, onignore }: Props = $props();
	const LABELS: Record<string, string> = {
		min_width_top_surface: 'Top surface minimum width'
	};
	const show = (v: unknown) => (Array.isArray(v) ? v[0] : v) ?? '—';
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-scrim" role="presentation">
	<div
		class="ws-dialog w-[30rem] max-w-[calc(100vw-2rem)] p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="handy-sugg-title"
		data-testid="handy-suggestion"
		tabindex="-1"
		use:dialogFocus
		onkeydown={(e) => e.key === 'Escape' && onignore()}
	>
		<h2 id="handy-sugg-title" class="ws-dialog-title mb-1">
			{$t('Recommended setting for this test')}
		</h2>
		<p class="text-sm text-content-muted">{$locale && suggestionText(suggestion)}</p>
		<ul class="mt-3 flex flex-col gap-1 text-sm">
			{#each Object.entries(suggestion.apply) as [key, value] (key)}
				<li class="flex items-center justify-between gap-3 rounded border border-border px-2 py-1">
					<span>{LABELS[key] ? $t(LABELS[key]) : key}</span>
					<span class="text-content-muted tabular-nums">
						{show(values[key])} → <strong class="text-content">{show(value)}</strong>
					</span>
				</li>
			{/each}
		</ul>
		<div class="mt-4 flex justify-end gap-2">
			<button
				type="button"
				class="ws-btn ws-btn-secondary"
				data-testid="handy-suggestion-ignore"
				onclick={onignore}
			>
				{$t('Ignore')}
			</button>
			<button
				type="button"
				class="ws-btn ws-btn-primary"
				data-testid="handy-suggestion-apply"
				onclick={onapply}
			>
				{$t('Apply')}
			</button>
		</div>
	</div>
</div>
