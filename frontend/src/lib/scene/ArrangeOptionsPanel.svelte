<script lang="ts">
	// Panneau « Options d'arrangement » (lot 34) : miroir du menu ImGui
	// GLCanvas3D::_render_arrange_menu d'Orca 2.4.1 ancré au bouton Arrange :
	// espacement (slider + saisie, 0 = auto, min_spacing en séquentiel), rotation
	// auto (grise « Aligner sur l'axe Y »), multi-matériaux par plateau, zone de
	// calibration (selon GET …/arrange/options), Réinitialiser. Plus intuitif
	// qu'Orca : les deux portées « Tout arranger » / « Ce plateau » sont visibles,
	// et le plateau verrouillé désactive « Ce plateau » au lieu d'un refus serveur.
	import type { ArrangeOptionsInfo } from '$lib/api/types';
	import type { SavedArrangeOptions } from './arrange-options';
	import { clampSpacing, resetOptions, spacingFloor } from './arrange-options';

	interface Props {
		open: boolean;
		options: SavedArrangeOptions;
		info: ArrangeOptionsInfo | null;
		/** Plateau courant verrouillé : « Ce plateau » refusé (422 serveur). */
		activePlateLocked?: boolean;
		/** Arrangement en cours : boutons désactivés. */
		busy?: boolean;
		onchange: (options: SavedArrangeOptions) => void;
		onarrange: (scope: 'all' | 'plate') => void;
		onclose: () => void;
	}

	let {
		open,
		options,
		info,
		activePlateLocked = false,
		busy = false,
		onchange,
		onarrange,
		onclose
	}: Props = $props();

	// Bornes de repli (hors séquentiel) si GET …/arrange/options n'a pas répondu
	// (valeurs ArrangeSettings par défaut d'Orca, zone de calibration masquée).
	const LOCAL_BOUNDS: ArrangeOptionsInfo = {
		defaults: {
			spacing: 0,
			enable_rotation: false,
			allow_multi_materials_on_same_plate: true,
			avoid_extrusion_cali_region: true,
			align_to_y_axis: false
		},
		is_seq_print: false,
		min_spacing: 0,
		show_avoid_extrusion_cali_region: false
	};
	const floor = $derived(info ? spacingFloor(info) : 0);
	const shownSpacing = $derived(Math.max(options.spacing, floor));

	function setSpacing(value: number) {
		// Hors séquentiel (ou si les bornes serveur ne sont pas encore arrivées),
		// la borne locale 0–100 suffit : la saisie reste réactive.
		onchange({ ...options, spacing: clampSpacing(value, info ?? LOCAL_BOUNDS) });
	}
</script>

{#if open}
	<div
		class="ws-floating fixed top-24 left-1/2 z-40 flex w-80 -translate-x-1/2 flex-col gap-3 p-3 text-sm"
		data-testid="arrange-options-panel"
		role="dialog"
		aria-label="Options d'arrangement"
	>
		<div class="flex items-center justify-between">
			<span class="ws-section-title">Options d'arrangement</span>
			<button
				type="button"
				class="ws-icon-btn ws-icon-btn-xs"
				data-testid="arrange-options-close"
				onclick={onclose}
				aria-label="Fermer">✕</button
			>
		</div>

		<label class="flex items-center gap-2">
			<span class="w-24 shrink-0 text-content-muted">Espacement</span>
			<input
				type="range"
				min={floor}
				max="100"
				step="0.5"
				class="flex-1"
				data-testid="arrange-spacing"
				aria-label="Espacement (mm)"
				value={shownSpacing}
				oninput={(e) => setSpacing(Number(e.currentTarget.value))}
			/>
			<input
				type="number"
				min={floor}
				max="100"
				step="0.5"
				class="ws-field ws-field-sm w-16"
				data-testid="arrange-spacing-input"
				aria-label="Espacement (mm), saisie"
				value={shownSpacing}
				oninput={(e) => setSpacing(Number(e.currentTarget.value))}
			/>
		</label>
		<p class="-mt-2 text-xs text-content-subtle">
			{floor > 0 ? `Séquentiel : espacement minimum ${floor} mm.` : '0 = espacement automatique.'}
		</p>

		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				data-testid="arrange-rotation"
				checked={options.enableRotation}
				onchange={(e) => onchange({ ...options, enableRotation: e.currentTarget.checked })}
			/>
			<span>Rotation automatique pour l'arrangement</span>
		</label>

		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				data-testid="arrange-align-y"
				checked={options.alignToYAxis && !options.enableRotation}
				disabled={options.enableRotation}
				onchange={(e) => onchange({ ...options, alignToYAxis: e.currentTarget.checked })}
			/>
			<span class={options.enableRotation ? 'text-content-subtle' : ''}>Aligner sur l'axe Y</span>
		</label>

		<label class="flex items-center gap-2">
			<input
				type="checkbox"
				data-testid="arrange-multi-materials"
				checked={options.allowMultiMaterialsOnSamePlate}
				onchange={(e) =>
					onchange({ ...options, allowMultiMaterialsOnSamePlate: e.currentTarget.checked })}
			/>
			<span>Plusieurs matériaux sur le même plateau</span>
		</label>

		{#if info?.show_avoid_extrusion_cali_region}
			<label class="flex items-center gap-2">
				<input
					type="checkbox"
					data-testid="arrange-cali-region"
					checked={options.avoidExtrusionCaliRegion}
					onchange={(e) =>
						onchange({ ...options, avoidExtrusionCaliRegion: e.currentTarget.checked })}
				/>
				<span>Éviter la zone de calibration d'extrusion</span>
			</label>
		{/if}

		<div class="flex items-center justify-between gap-2 border-t border-border pt-2">
			<button
				type="button"
				class="ws-btn ws-btn-sm ws-btn-secondary"
				data-testid="arrange-reset"
				disabled={!info}
				onclick={() => info && onchange(resetOptions(info))}
			>
				Réinitialiser
			</button>
			<div class="flex gap-2">
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-secondary"
					data-testid="arrange-plate"
					disabled={busy || activePlateLocked}
					title={activePlateLocked
						? 'Plateau verrouillé : déverrouillez-le pour l’arranger.'
						: 'Arranger uniquement les objets du plateau courant'}
					onclick={() => onarrange('plate')}
				>
					Ce plateau
				</button>
				<button
					type="button"
					class="ws-btn ws-btn-sm ws-btn-primary"
					data-testid="arrange-all"
					disabled={busy}
					onclick={() => onarrange('all')}
				>
					Tout arranger
				</button>
			</div>
		</div>
	</div>
{/if}
