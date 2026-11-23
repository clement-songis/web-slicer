<script lang="ts">
	// Outil oreilles de bord (T057 ; audit 45 EARS-b = GLGizmoBrimEars d'Orca) :
	// diamètre de tête (5 – 20), angle max et rayon de détection de
	// l'auto-génération, « Auto-générer », retrait de la sélection / de tout,
	// vue en coupe, avertissements « type de bordure non peint » (+ lien qui le
	// règle sur l'objet) et « oreilles invalides ». Présentationnel : la page
	// pose les points par clic sur le modèle et calcule la 1ʳᵉ couche.
	import type { BrimEarPoint } from './brim-ears';
	import { HEAD_DIAMETER_MAX, HEAD_DIAMETER_MIN } from './brim-ears';
	import { fmtNum, locale, t } from '$lib/i18n';
	import IconX from '~icons/lucide/x';

	interface Props {
		/** Oreilles de l'objet, positions plateau (XY) et rayon. */
		points: readonly (BrimEarPoint & { wx: number; wy: number })[];
		/** Indices sélectionnés (liste ou clic). */
		selected?: ReadonlySet<number>;
		/** Indices des oreilles invalides (find_single). */
		invalid?: readonly number[];
		headDiameter: number;
		maxAngle: number;
		detectionRadius: number;
		detectionRadiusMax?: number;
		section?: number;
		/** Type de bordure effectif de l'objet = « painted ». */
		brimPainted: boolean;
		onremove: (index: number) => void;
		onclear?: () => void;
		onremoveselected?: () => void;
		onselect?: (index: number, additive: boolean) => void;
		onheaddiameter?: (v: number) => void;
		onmaxangle?: (v: number) => void;
		ondetectionradius?: (v: number) => void;
		onsection?: (v: number) => void;
		onautogenerate?: () => void;
		onsetpainted?: () => void;
		ondone?: () => void;
	}

	let {
		points,
		selected = new Set<number>(),
		invalid = [],
		headDiameter,
		maxAngle,
		detectionRadius,
		detectionRadiusMax = 100,
		section = 0,
		brimPainted,
		onremove,
		onclear,
		onremoveselected,
		onselect,
		onheaddiameter,
		onmaxangle,
		ondetectionradius,
		onsection,
		onautogenerate,
		onsetpainted,
		ondone
	}: Props = $props();

	const num = (raw: string, min: number, max: number): number | null => {
		const v = Number(raw);
		return Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : null;
	};
	const invalidSet = $derived(new Set(invalid));
</script>

{#snippet slider(
	label: string,
	testid: string,
	value: number,
	min: number,
	max: number,
	step: number,
	digits: number,
	onset: ((v: number) => void) | undefined
)}
	<label class="grid grid-cols-[7.5rem_1fr_4rem] items-center gap-2">
		<span class="text-content-muted">{label}</span>
		<input
			type="range"
			{min}
			{max}
			{step}
			{value}
			aria-label={label}
			data-testid={`${testid}-slider`}
			oninput={(e) => {
				const v = num(e.currentTarget.value, min, max);
				if (v !== null) onset?.(v);
			}}
		/>
		<input
			type="number"
			class="ws-field ws-field-sm w-16 tabular-nums"
			{min}
			{max}
			{step}
			value={Number(value.toFixed(digits))}
			aria-label={label}
			data-testid={testid}
			onchange={(e) => {
				const v = num(e.currentTarget.value, min, max);
				if (v !== null) onset?.(v);
			}}
		/>
	</label>
{/snippet}

<div class="flex flex-col gap-2 text-sm" data-testid="brim-ears-tool">
	{@render slider(
		$t('Head diameter'),
		'brim-ears-head',
		headDiameter,
		HEAD_DIAMETER_MIN,
		HEAD_DIAMETER_MAX,
		0.1,
		1,
		onheaddiameter
	)}
	<hr class="border-border" />
	{@render slider($t('Max angle'), 'brim-ears-angle', maxAngle, 0, 180, 0.1, 1, onmaxangle)}
	{@render slider(
		$t('Detection radius'),
		'brim-ears-radius',
		detectionRadius,
		0,
		Math.max(1, Math.floor(detectionRadiusMax)),
		0.1,
		1,
		ondetectionradius
	)}
	<hr class="border-border" />
	<div class="grid grid-cols-[7.5rem_1fr] items-center gap-2">
		<span class="text-content-muted">{$t('Create')}</span>
		<button
			type="button"
			class="ws-btn ws-btn-sm ws-btn-secondary justify-self-start"
			data-testid="brim-ears-auto"
			title={$t('Generate brim ears using Max angle and Detection radius')}
			onclick={() => onautogenerate?.()}
		>
			{$t('Auto-generate')}
		</button>
		<span class="text-content-muted">{$t('Remove')}</span>
		<span class="flex gap-2">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary"
				data-testid="brim-ears-remove-selected"
				disabled={selected.size === 0}
				onclick={() => onremoveselected?.()}
			>
				{$t('Selected')}
			</button>
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary"
				data-testid="brim-ears-remove-all"
				disabled={points.length === 0}
				onclick={() => onclear?.()}
			>
				{$t('All')}
			</button>
		</span>
	</div>
	<hr class="border-border" />
	{@render slider($t('Section view'), 'brim-ears-section', section, 0, 1, 0.01, 2, onsection)}
	<hr class="border-border" />
	{#if points.length === 0}
		<p class="text-content-subtle">{$t('Click the model to place brim ears.')}</p>
	{:else}
		<ul class="flex max-h-48 flex-col gap-1 overflow-y-auto" data-testid="brim-ears-list">
			{#each points as p, i (i)}
				<li
					class="flex items-center justify-between rounded px-2 py-0.5 {selected.has(i)
						? 'bg-primary/15 ring-1 ring-primary'
						: 'bg-surface-sunken'}"
					data-testid="brim-ear-row"
					data-invalid={invalidSet.has(i) ? 'true' : undefined}
				>
					<button
						type="button"
						class="flex-1 text-left {invalidSet.has(i) ? 'text-danger' : ''}"
						aria-pressed={selected.has(i)}
						onclick={(e) => onselect?.(i, e.shiftKey || e.ctrlKey || e.metaKey)}
					>
						X {fmtNum(p.wx, 1, $locale)} · Y {fmtNum(p.wy, 1, $locale)} · Ø {fmtNum(
							p.r * 2,
							1,
							$locale
						)}
					</button>
					<button
						type="button"
						class="ws-icon-btn ws-icon-btn-xs"
						aria-label={$t('Remove ear {n}', { n: i + 1 })}
						onclick={() => onremove(i)}
					>
						<IconX class="h-3.5 w-3.5" aria-hidden="true" />
					</button>
				</li>
			{/each}
		</ul>
	{/if}
	{#if !brimPainted || invalid.length}
		<div
			class="flex flex-col gap-1 text-xs text-warning"
			role="status"
			data-testid="brim-ears-warning"
		>
			{#if !brimPainted}
				<p>
					{$t(
						'Warning: The brim type is not set to "painted", the brim ears will not take effect!'
					)}
				</p>
				<button
					type="button"
					class="self-start font-semibold text-primary underline"
					data-testid="brim-ears-set-painted"
					onclick={() => onsetpainted?.()}
				>
					{$t('Set the brim type of this object to "painted"')}
				</button>
			{/if}
			{#if invalid.length}
				<p data-testid="brim-ears-invalid">
					{$t('Warning')}: {invalid.length}
					{$t('invalid brim ears')}
				</p>
			{/if}
		</div>
	{/if}
	<button type="button" class="ws-btn ws-btn-sm ws-btn-primary self-end" onclick={() => ondone?.()}>
		{$t('Done')}
	</button>
</div>
