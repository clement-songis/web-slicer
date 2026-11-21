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
			class="ws-dialog flex w-full max-w-lg flex-col gap-3 p-4"
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
			<div class="flex flex-col divide-y divide-border">
				{#each visible as key (key)}
					{@const def = PARAMS[key]}
					<label class="flex items-center justify-between gap-2 py-1.5 text-sm" data-param={key}>
						<span class="text-content-muted">{$t(def.label)}</span>
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
							<select class="ws-field ws-field-sm w-48 shrink-0" bind:value={draft[key]}>
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
			<div class="flex justify-end gap-2">
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
