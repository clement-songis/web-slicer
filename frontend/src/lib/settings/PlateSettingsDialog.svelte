<script lang="ts">
	import {
		FILAMENT_MAP_MODES,
		nozzleLabel,
		readFilamentMap,
		readFilamentMapMode,
		writeFilamentMap,
		type FilamentMapMode
	} from '$lib/editor/filament-map';
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
	}

	let {
		open = false,
		settings = {},
		mode = $bindable('simple'),
		onClose,
		onsetting,
		extruders = 1,
		filamentColours = []
	}: Props = $props();

	// Répartition par buse (Orca FilamentMapDialog) : mode + buse de chaque filament.
	let mapMode = $state<FilamentMapMode | ''>('');
	let map = $state<number[]>([]);
	$effect(() => {
		if (!open) return;
		mapMode = readFilamentMapMode(settings?.filament_map_mode);
		map = readFilamentMap(settings?.filament_map, filamentColours.length, extruders);
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
	/** Libellés FR des modes, identiques au dialogue de réglages (SettingsTabs). */
	const MODE_LABELS: Record<DisplayMode, string> = {
		simple: 'Simple',
		advanced: 'Avancé',
		expert: 'Expert',
		develop: 'Développeur'
	};

	let draft = $state<Record<string, string>>({});
	// Ouverture : brouillon = surcharges actuelles (texte Orca).
	$effect(() => {
		if (!open) return;
		const next: Record<string, string> = {};
		for (const key of PLATE_KEYS) {
			const v = settings?.[key];
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
			aria-label="Réglages du plateau"
			data-testid="settings-dialog"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && onClose()}
			use:dialogFocus={onClose}
		>
			<h2 class="ws-dialog-title">Réglages du plateau</h2>
			<div class="ws-seg flex w-full" role="group" aria-label="Mode d'affichage">
				{#each MODES as m (m)}
					<button
						type="button"
						data-testid={`settings-mode-${m}`}
						aria-pressed={mode === m}
						class="ws-seg-item flex-1 {mode === m ? 'ws-seg-on' : ''}"
						onclick={() => (mode = m)}>{MODE_LABELS[m]}</button
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
								<option value="">Profil</option>
								<option value="1">Oui</option>
								<option value="0">Non</option>
							</select>
						{:else if isOpenEnum(def)}
							<input
								type="text"
								class="ws-field ws-field-sm w-48 shrink-0"
								placeholder="Profil"
								bind:value={draft[key]}
							/>
						{:else}
							<select class="ws-field ws-field-sm w-48 shrink-0" bind:value={draft[key]}>
								<option value="">Profil</option>
								{#each def.enumValues as v, i (v)}
									<option value={v}>{$t(def.enumLabels[i] || v)}</option>
								{/each}
							</select>
						{/if}
					</label>
					{#if key === 'print_sequence' && draft[key] === 'by object'}
						<p class="pb-1.5 text-xs text-content-subtle" data-testid="plate-sequence-hint">
							Objet par objet : l'ordre de la liste d'objets est l'ordre d'impression (glissez un
							objet sur un autre pour le changer).
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
						<h3 class="ws-section-title">Filaments par buse</h3>
						<select
							class="ws-field ws-field-sm w-60"
							data-testid="plate-filament-map-mode"
							aria-label="Mode de répartition"
							bind:value={mapMode}
						>
							<option value="">Profil</option>
							{#each FILAMENT_MAP_MODES as m (m.value)}
								<option value={m.value}>{m.label}</option>
							{/each}
						</select>
					</div>
					<p class="text-xs text-content-subtle">
						{FILAMENT_MAP_MODES.find((m) => m.value === mapMode)?.hint ??
							'Suit le mode enregistré dans le profil (automatique, moins de purge, par défaut).'}
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
										Filament {i + 1}
									</span>
									<span class="ws-seg" role="radiogroup" aria-label="Buse du filament {i + 1}">
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
					onclick={apply}>Enregistrer</button
				>
			</div>
		</div>
	</div>
{/if}
