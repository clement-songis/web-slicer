<script lang="ts">
	import {
		FILAMENT_MAP_MODES,
		nozzleLabel,
		readFilamentMap,
		readFilamentMapMode,
		writeFilamentMap,
		type FilamentMapMode
	} from '$lib/editor/filament-map';
	import { untrack } from 'svelte';
	import { swatchStyle } from '$lib/ui/color';
	import { dialogFocus } from '$lib/ui/dialog-focus';
	// Dialogue « Réglages du plateau » (parité Orca PlateSettingsDialog, ouvert
	// par l'icône du plateau) : type de plaque, séquence d'impression, séquence
	// des couches, vase spirale. Brouillon local appliqué par « Enregistrer » ;
	// une valeur vide = suivre le profil (pas de surcharge du plateau).
	import { PARAMS } from '../../generated/params';
	import { t } from '$lib/i18n';
	import { modeAllows, type DisplayMode } from './filter';
	import { isOpenEnum } from './widgets/types';
	import FilamentOrderEditor from './FilamentOrderEditor.svelte';
	import {
		MAX_LAYER_VALUE,
		MIN_LAYER_VALUE,
		defaultOrder,
		formatOrder,
		formatOtherLayersSequence,
		newRange,
		normalizeOrder,
		parseFirstLayerSequence,
		parseOtherLayersSequence,
		rangeValid,
		sortRanges,
		type LayerSeqRange
	} from './plate-sequence';

	interface Props {
		open?: boolean;
		/** Surcharges actuelles du plateau (clé → valeur Orca). */
		settings?: Record<string, unknown>;
		mode?: DisplayMode;
		onClose: () => void;
		/** Applique une surcharge (`undefined` = retour au profil). */
		onsetting: (key: string, value: unknown) => void;
		/** Nombre de buses (H2D : 2) ; > 1 → répartition des filaments par buse. */
		extruders?: number;
		/** Couleur de chaque filament de la barre (pastilles de la répartition). */
		filamentColours?: readonly string[];
		/** Valeurs des presets : option « Profil (…) » (Orca « Same as Global … »). */
		inherited?: Record<string, unknown>;
	}

	let {
		open = false,
		settings = {},
		mode = $bindable('simple'),
		onClose,
		onsetting,
		extruders = 1,
		filamentColours = [],
		inherited = {}
	}: Props = $props();

	/** « Profil (Par couche) » : valeur suivie quand le plateau ne surcharge rien. */
	function profileLabel(key: string): string {
		const def = PARAMS[key];
		let v = inherited[key];
		if (Array.isArray(v)) v = v[0];
		if (!def || v === undefined || v === null || v === '') return $t('Profile');
		if (def.type === 'coBool')
			return $t('Profile ({value})', {
				value: v === true || v === '1' || v === 1 ? $t('Yes') : $t('No')
			});
		const i = def.enumValues.indexOf(String(v));
		return i >= 0
			? $t('Profile ({value})', { value: $t(def.enumLabels[i] || def.enumValues[i]) })
			: $t('Profile');
	}

	// Répartition par buse (Orca FilamentMapDialog) : mode + buse de chaque filament.
	// Brouillon pris **à l'ouverture** seulement : les props sont recalculées par
	// la page (nouvel objet `settings`, nouveau tableau de couleurs à chaque
	// rechargement d'un preset) ; les suivre effaçait le choix en cours (phase 41d).
	let mapMode = $state<FilamentMapMode | ''>('');
	let map = $state<number[]>([]);
	$effect(() => {
		if (!open) return;
		untrack(() => {
			mapMode = readFilamentMapMode(settings?.filament_map_mode);
			map = readFilamentMap(settings?.filament_map, filamentColours.length, extruders);
		});
	});
	// Dialogue ouvert : un filament ajouté / retiré redimensionne la carte sans
	// perdre les buses déjà choisies.
	$effect(() => {
		const n = filamentColours.length;
		const k = extruders;
		if (!open) return;
		untrack(() => {
			if (map.length !== n || map.some((v) => v > k)) map = readFilamentMap(map, n, k);
		});
	});
	const showMap = $derived(extruders > 1 && filamentColours.length > 0);

	// Audit 45 MM-b — « Personnaliser » (Orca PlateSettingsDialog) : ordre des
	// filaments de la 1ʳᵉ couche (DragCanvas) et plages « Couche … à … » des
	// autres couches avec leur ordre (OtherLayersSeqPanel). Brouillon pris à
	// l'ouverture, écrit par « Enregistrer » (first_layer_print_sequence,
	// other_layers_print_sequence[_nums]) ; « Auto » les retire.
	let firstOrder = $state<number[]>([]);
	let otherRanges = $state<LayerSeqRange[]>([]);
	let rangeError = $state<string | null>(null);
	$effect(() => {
		if (!open) return;
		untrack(() => {
			const n = filamentColours.length;
			firstOrder =
				parseFirstLayerSequence(settings?.first_layer_print_sequence, n) ?? defaultOrder(n);
			const ranges = parseOtherLayersSequence(
				settings?.other_layers_print_sequence,
				settings?.other_layers_print_sequence_nums,
				n
			);
			otherRanges = ranges.length ? ranges : [newRange(0, n)];
			rangeError = null;
		});
	});
	// Filament ajouté / retiré dialogue ouvert : ordres complétés.
	$effect(() => {
		const n = filamentColours.length;
		if (!open) return;
		untrack(() => {
			if (firstOrder.length !== n) firstOrder = normalizeOrder(firstOrder, n);
			if (otherRanges.some((r) => r.order.length !== n))
				otherRanges = otherRanges.map((r) => ({ ...r, order: normalizeOrder(r.order, n) }));
		});
	});
	function setRange(i: number, patch: Partial<LayerSeqRange>) {
		otherRanges = otherRanges.map((r, k) => (k === i ? { ...r, ...patch } : r));
		rangeError = null;
	}
	function setBegin(i: number, raw: string) {
		const v = Math.max(MIN_LAYER_VALUE, Math.round(Number(raw)) || MIN_LAYER_VALUE);
		const r = otherRanges[i];
		// LayerNumberTextInput : fin repoussée au début si elle le précède.
		setRange(i, { begin: v, end: r.end < v ? v : r.end });
		otherRanges = sortRanges(otherRanges);
	}
	function setEnd(i: number, raw: string) {
		const r = otherRanges[i];
		const v = Math.max(MIN_LAYER_VALUE, Math.round(Number(raw)) || MIN_LAYER_VALUE);
		// Orca : une fin inférieure au début est refusée (valeur précédente).
		if (v < r.begin) return;
		setRange(i, { end: v });
		otherRanges = sortRanges(otherRanges);
	}

	/** Options du dialogue Orca, dans son ordre. */
	const PLATE_KEYS = [
		'curr_bed_type',
		'print_sequence',
		'first_layer_sequence_choice',
		'other_layers_sequence_choice',
		'spiral_mode'
	] as const;
	const MODES: DisplayMode[] = ['simple', 'advanced', 'expert', 'develop'];
	/** Libellés des modes (clés i18n), identiques au dialogue de réglages (SettingsTabs). */
	const MODE_LABELS: Record<DisplayMode, string> = {
		simple: 'Simple',
		advanced: 'Advanced',
		expert: 'Expert',
		develop: 'Developer'
	};

	let draft = $state<Record<string, string>>({});
	// Ouverture : brouillon = surcharges actuelles (texte Orca).
	$effect(() => {
		if (!open) return;
		const current = untrack(() => settings);
		const next: Record<string, string> = {};
		for (const key of PLATE_KEYS) {
			const v = current?.[key];
			next[key] =
				v === undefined || v === null ? '' : typeof v === 'boolean' ? (v ? '1' : '0') : String(v);
		}
		draft = next;
	});

	const visible = $derived(
		PLATE_KEYS.filter((key) => PARAMS[key] && modeAllows(PARAMS[key].mode, mode))
	);

	function apply() {
		const firstCustom = draft.first_layer_sequence_choice === 'Customize';
		const otherCustom = draft.other_layers_sequence_choice === 'Customize';
		if (otherCustom && !otherRanges.every(rangeValid)) {
			rangeError = $t('Please input layer value (>= 2).');
			return;
		}
		for (const key of PLATE_KEYS) {
			const before = settings?.[key];
			const was = before === undefined || before === null ? '' : String(before);
			if (draft[key] !== was) onsetting(key, draft[key] === '' ? undefined : draft[key]);
		}
		if (showMap) {
			const beforeMode = readFilamentMapMode(settings?.filament_map_mode);
			if (mapMode !== beforeMode) onsetting('filament_map_mode', mapMode || undefined);
			const next = mapMode === 'Manual' ? writeFilamentMap(map) : undefined;
			const prev = settings?.filament_map;
			if (JSON.stringify(next ?? null) !== JSON.stringify(prev ?? null))
				onsetting('filament_map', next);
		}
		// Séquences (Plater : set_first_layer_print_sequence / set_other_layers_…,
		// vides quand le choix est « Auto »).
		const writeIfChanged = (key: string, value: string | undefined) => {
			const before = settings?.[key];
			const was = before === undefined || before === null ? undefined : String(before);
			if (was !== value) onsetting(key, value);
		};
		writeIfChanged(
			'first_layer_print_sequence',
			firstCustom && firstOrder.length ? formatOrder(firstOrder) : undefined
		);
		const other = otherCustom && otherRanges.length ? formatOtherLayersSequence(otherRanges) : null;
		writeIfChanged('other_layers_print_sequence', other?.sequence);
		writeIfChanged('other_layers_print_sequence_nums', other?.nums);
		onClose();
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
		role="presentation"
		onclick={onClose}
	>
		<div
			class="ws-dialog flex max-h-full w-full max-w-lg flex-col gap-3 p-4"
			role="dialog"
			aria-modal="true"
			aria-label={$t('Plate settings')}
			data-testid="settings-dialog"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onClose()}
			use:dialogFocus={onClose}
		>
			<h2 class="ws-dialog-title">{$t('Plate settings')}</h2>
			<div class="ws-seg flex w-full" role="group" aria-label={$t('Display mode')}>
				{#each MODES as m (m)}
					<button
						type="button"
						data-testid={`settings-mode-${m}`}
						aria-pressed={mode === m}
						class="ws-seg-item flex-1 {mode === m ? 'ws-seg-on' : ''}"
						onclick={() => (mode = m)}>{$t(MODE_LABELS[m])}</button
					>
				{/each}
			</div>
			<!-- Audit 46 PLATE-a : corps défilant (OtherLayersSeqPanel d'Orca =
			     wxScrolledWindow), titre et boutons toujours visibles en 800×500. -->
			<div
				class="-mx-1 flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-1"
				data-testid="plate-settings-body"
			>
				<div class="flex flex-col divide-y divide-border">
					{#each visible as key (key)}
						{@const def = PARAMS[key]}
						<label
							class="flex flex-wrap items-center justify-between gap-2 py-1.5 text-sm"
							data-param={key}
						>
							<span class="min-w-0 text-content-muted">{$t(def.label)}</span>
							{#if def.type === 'coBool'}
								<select class="ws-field ws-field-sm w-48 shrink-0" bind:value={draft[key]}>
									<option value="">{profileLabel(key)}</option>
									<option value="1">{$t('Yes')}</option>
									<option value="0">{$t('No')}</option>
								</select>
							{:else if isOpenEnum(def)}
								<input
									type="text"
									class="ws-field ws-field-sm w-48 shrink-0"
									placeholder={$t('Profile')}
									bind:value={draft[key]}
								/>
							{:else}
								<select
									class="ws-field ws-field-sm w-48 max-w-full min-w-0"
									bind:value={draft[key]}
								>
									<option value="">{profileLabel(key)}</option>
									{#each def.enumValues as v, i (v)}
										<option value={v}>{$t(def.enumLabels[i] || v)}</option>
									{/each}
								</select>
							{/if}
						</label>
						{#if key === 'print_sequence' && draft[key] === 'by object'}
							<p class="pb-1.5 text-xs text-content-subtle" data-testid="plate-sequence-hint">
								{$t(
									'By object: the order of the object list is the print order (drag an object onto another to change it).'
								)}
							</p>
						{/if}
						{#if key === 'first_layer_sequence_choice' && draft[key] === 'Customize' && filamentColours.length}
							<div class="flex flex-col gap-1 pb-2" data-testid="plate-first-layer-sequence">
								<span class="text-xs text-content-subtle"
									>{$t('Drag the filaments to set their print order on the first layer.')}</span
								>
								<FilamentOrderEditor
									order={firstOrder}
									colours={filamentColours}
									label={$t('First layer filament sequence')}
									testid="plate-first-layer-order"
									onchange={(o) => (firstOrder = o)}
								/>
							</div>
						{/if}
						{#if key === 'other_layers_sequence_choice' && draft[key] === 'Customize' && filamentColours.length}
							<div class="flex flex-col gap-2 pb-2" data-testid="plate-other-layers-sequence">
								{#each otherRanges as range, i (i)}
									<div
										class="flex flex-wrap items-center gap-2 text-sm"
										data-testid="plate-layer-range"
									>
										<span class="text-content-muted">{$t('Layer')}</span>
										<input
											type="number"
											min={MIN_LAYER_VALUE}
											step="1"
											class="ws-field ws-field-sm w-16 tabular-nums"
											aria-label={$t('First layer of range {n}', { n: i + 1 })}
											data-testid="plate-layer-range-begin"
											value={range.begin}
											onchange={(e) => setBegin(i, e.currentTarget.value)}
										/>
										<span class="text-content-muted">{$t('to')}</span>
										<select
											class="ws-field ws-field-sm w-28"
											aria-label={$t('Last layer of range {n}', { n: i + 1 })}
											data-testid="plate-layer-range-end-kind"
											value={range.end === MAX_LAYER_VALUE ? 'end' : 'custom'}
											onchange={(e) =>
												setRange(i, {
													end:
														e.currentTarget.value === 'end'
															? MAX_LAYER_VALUE
															: Math.max(range.begin, MIN_LAYER_VALUE)
												})}
										>
											<option value="end">{$t('End')}</option>
											<option value="custom">{$t('Customize')}</option>
										</select>
										{#if range.end !== MAX_LAYER_VALUE}
											<input
												type="number"
												min={range.begin}
												step="1"
												class="ws-field ws-field-sm w-16 tabular-nums"
												aria-label={$t('Last layer of range {n}', { n: i + 1 })}
												data-testid="plate-layer-range-end"
												value={range.end}
												onchange={(e) => setEnd(i, e.currentTarget.value)}
											/>
										{/if}
										<FilamentOrderEditor
											order={range.order}
											colours={filamentColours}
											label={$t('Filament sequence of layers {a} to {b}', {
												a: range.begin,
												b: range.end === MAX_LAYER_VALUE ? $t('End') : range.end
											})}
											testid="plate-layer-range-order"
											onchange={(o) => setRange(i, { order: o })}
										/>
									</div>
								{/each}
								<div class="flex gap-2">
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										data-testid="plate-layer-range-add"
										onclick={() =>
											(otherRanges = [
												...otherRanges,
												newRange(otherRanges.length, filamentColours.length)
											])}>{$t('Add')}</button
									>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-ghost"
										data-testid="plate-layer-range-delete"
										disabled={otherRanges.length <= 1}
										onclick={() => (otherRanges = otherRanges.slice(0, -1))}>{$t('Delete')}</button
									>
								</div>
								{#if rangeError}
									<p class="text-xs text-danger" role="alert">{rangeError}</p>
								{/if}
							</div>
						{/if}
					{/each}
				</div>
				{#if showMap}
					<section
						class="flex flex-col gap-2 border-t border-border pt-3"
						data-testid="plate-filament-map"
					>
						<div class="flex items-center justify-between gap-2">
							<h3 class="ws-section-title">{$t('Filaments per nozzle')}</h3>
							<select
								class="ws-field ws-field-sm w-60"
								data-testid="plate-filament-map-mode"
								aria-label={$t('Grouping mode')}
								bind:value={mapMode}
							>
								<option value="">{$t('Profile')}</option>
								{#each FILAMENT_MAP_MODES as m (m.value)}
									<option value={m.value}>{$t(m.label)}</option>
								{/each}
							</select>
						</div>
						<p class="text-xs text-content-subtle">
							{$t(
								FILAMENT_MAP_MODES.find((m) => m.value === mapMode)?.hint ??
									'Follows the mode saved in the profile (automatic, less flushing, default).'
							)}
						</p>
						{#if mapMode === 'Manual'}
							<ul class="flex flex-col gap-1.5" data-testid="plate-filament-map-list">
								{#each map as nozzle, i (i)}
									<li class="flex items-center justify-between gap-2 text-sm">
										<span class="flex items-center gap-2">
											<span
												class="h-4 w-4 rounded border border-border-strong"
												style={swatchStyle(filamentColours[i])}
											></span>
											{$t('Filament {n}', { n: i + 1 })}
										</span>
										<span
											class="ws-seg"
											role="radiogroup"
											aria-label={$t('Nozzle of filament {n}', { n: i + 1 })}
										>
											{#each Array.from({ length: extruders }, (_, k) => k + 1) as n (n)}
												<button
													type="button"
													role="radio"
													aria-checked={nozzle === n}
													data-testid="plate-filament-map-{i + 1}-{n}"
													class="ws-seg-item {nozzle === n ? 'ws-seg-on' : ''}"
													onclick={() => (map[i] = n)}>{nozzleLabel(n, extruders)}</button
												>
											{/each}
										</span>
									</li>
								{/each}
							</ul>
						{/if}
					</section>
				{/if}
			</div>
			<div class="flex shrink-0 justify-end gap-2">
				<button type="button" class="ws-btn ws-btn-secondary" onclick={onClose}
					>{$t('Close')}</button
				>
				<button
					type="button"
					data-testid="settings-apply"
					class="ws-btn ws-btn-primary"
					onclick={apply}>{$t('Save')}</button
				>
			</div>
		</div>
	</div>
{/if}
