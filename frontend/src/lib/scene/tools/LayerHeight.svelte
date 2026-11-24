<script lang="ts">
	// Éditeur de hauteur de couche variable (T058, toolbar `layersediting`) :
	// visualise le profil, permet d'imposer une hauteur sur une bande Z, de lisser
	// et de réinitialiser. Présentationnel — émet le nouveau profil au parent
	// (qui persiste dans le document scène et rafraîchit l'aperçu).
	import {
		DEFAULT_MAX_HEIGHT,
		DEFAULT_MIN_HEIGHT,
		heightAt,
		setBand,
		smooth,
		uniformProfile,
		type LayerBand
	} from './layer-height';
	import { LayerBrushDriver, actionFor } from './layer-brush';
	import { t } from '#lib/i18n';

	interface Props {
		profile: LayerBand[];
		zMax: number;
		/** Hauteur de couche par défaut (réinitialisation). */
		baseHeight?: number;
		onchange: (profile: LayerBand[]) => void;
		/** Profil adaptatif calculé par le moteur (libslic3r, `quality` ∈ [0, 1]). */
		onadaptive?: (quality: number) => Promise<void>;
		/** Lissage gaussien calculé par le moteur (`smooth_height_profile`). */
		onsmooth?: (radius: number, keepMin: boolean) => Promise<void>;
		/** Pinceau moteur (`adjust_layer_height_profile`) à la cote `z`. */
		onbrush?: (
			z: number,
			action: 'increase' | 'decrease' | 'reduce' | 'smooth',
			delta: number,
			bandWidth: number
		) => Promise<void>;
	}

	let {
		profile,
		zMax,
		baseHeight = 0.2,
		onchange,
		onadaptive,
		onsmooth,
		onbrush
	}: Props = $props();
	// Pinceau (barre de profil, comme la barre d'épaisseur d'Orca) : bande 2 mm
	// (défaut GUI, 1,5–10) ; 0,005 par évènement (appui, glisser, minuteur 300 ms).
	// Clic gauche = affiner, droit = épaissir, Maj = réduire / lisser.
	let brushBand = $state(2);
	const brush = new LayerBrushDriver((z, action, delta) =>
		onbrush ? onbrush(z, action, delta, brushBand) : Promise.resolve()
	);
	$effect(() => () => brush.stop());
	function zAt(event: PointerEvent): number {
		const rect = (event.currentTarget as Element).getBoundingClientRect();
		const t = 1 - (event.clientY - rect.top) / (rect.height || 1);
		return Math.min(zMax, Math.max(0, t * zMax));
	}
	function onBarDown(event: PointerEvent) {
		if (!onbrush) return;
		event.preventDefault();
		brush.start(zAt(event), actionFor(event.button === 2, event.shiftKey));
		window.addEventListener('pointerup', () => brush.stop(), { once: true });
	}
	function onBarMove(event: PointerEvent) {
		if (brush.active) brush.move(zAt(event));
	}
	// Clic droit = épaissir (barre d'Orca) : jamais le menu de la scène
	// (audit 46 PREF-c). Écouteur natif : `oncontextmenu` est délégué par
	// Svelte à la racine, où le gestionnaire de la vue le recevrait aussi.
	function swallowContextMenu(node: Element, enabled: boolean) {
		let active = enabled;
		const stop = (e: Event) => {
			if (!active) return;
			e.preventDefault();
			e.stopPropagation();
		};
		node.addEventListener('contextmenu', stop);
		return {
			update: (next: boolean) => (active = next),
			destroy: () => node.removeEventListener('contextmenu', stop)
		};
	}

	// Défauts de la GUI d'Orca : qualité 0,5, rayon 5, « keep min » décoché.
	let quality = $state(0.5);
	let radius = $state(5);
	let keepMin = $state(false);
	let busy = $state(false);
	async function run(task: () => Promise<void>) {
		busy = true;
		try {
			await task();
		} finally {
			busy = false;
		}
	}

	let z0 = $state(0);
	let z1 = $state(0);
	let bandHeight = $state(0.2);

	// Échantillonne le profil pour la courbe SVG (z vertical, hauteur horizontale).
	const samples = $derived.by(() => {
		const n = 40;
		const pts: string[] = [];
		for (let i = 0; i <= n; i++) {
			const z = (zMax * i) / n;
			const h = heightAt(profile, z);
			const x = ((h - DEFAULT_MIN_HEIGHT) / (DEFAULT_MAX_HEIGHT - DEFAULT_MIN_HEIGHT)) * 100;
			const y = 100 - (z / (zMax || 1)) * 100;
			pts.push(`${x.toFixed(1)},${y.toFixed(1)}`);
		}
		return pts.join(' ');
	});
</script>

<div class="flex gap-3 text-sm">
	<svg
		viewBox="0 0 100 100"
		class="h-40 w-16 rounded border border-border bg-surface-raised"
		class:cursor-crosshair={!!onbrush}
		role="slider"
		aria-label={$t('Layer height brush (left thins, right thickens, Shift reduces/smooths)')}
		aria-valuemin={0}
		aria-valuemax={zMax}
		aria-valuenow={0}
		tabindex="-1"
		onpointerdown={onBarDown}
		onpointermove={onBarMove}
		use:swallowContextMenu={!!onbrush}
	>
		<polyline
			points={samples}
			fill="none"
			stroke="currentColor"
			class="text-accent"
			stroke-width="1.5"
		/>
	</svg>

	<div class="flex flex-1 flex-col gap-2">
		<div class="grid grid-cols-3 gap-1">
			<label class="flex flex-col">
				<span class="text-content-muted">{$t('Z from')}</span>
				<input
					type="number"
					min={0}
					max={zMax}
					step={0.1}
					bind:value={z0}
					class="ws-field ws-field-sm"
				/>
			</label>
			<label class="flex flex-col">
				<span class="text-content-muted">{$t('Z to')}</span>
				<input
					type="number"
					min={0}
					max={zMax}
					step={0.1}
					bind:value={z1}
					class="ws-field ws-field-sm"
				/>
			</label>
			<label class="flex flex-col">
				<span class="text-content-muted">{$t('Height')}</span>
				<input
					type="number"
					min={DEFAULT_MIN_HEIGHT}
					max={DEFAULT_MAX_HEIGHT}
					step={0.01}
					bind:value={bandHeight}
					class="ws-field ws-field-sm"
				/>
			</label>
		</div>

		{#if onbrush}
			<label class="flex flex-col">
				<span class="text-content-muted">{$t('Brush band (mm) — on the curve')}</span>
				<input
					type="number"
					min={1.5}
					max={10}
					step={0.5}
					bind:value={brushBand}
					class="ws-field ws-field-sm"
				/>
			</label>
		{/if}

		{#if onadaptive || onsmooth}
			<div class="grid grid-cols-3 gap-1">
				<label class="flex flex-col">
					<span class="text-content-muted">{$t('Quality')}</span>
					<input
						type="number"
						min={0}
						max={1}
						step={0.05}
						bind:value={quality}
						class="ws-field ws-field-sm"
					/>
				</label>
				<label class="flex flex-col">
					<span class="text-content-muted">{$t('Radius')}</span>
					<input
						type="number"
						min={1}
						max={10}
						step={1}
						bind:value={radius}
						class="ws-field ws-field-sm"
					/>
				</label>
				<label class="flex items-end gap-1">
					<input type="checkbox" bind:checked={keepMin} />
					<span class="text-content-muted">{$t('Keep min')}</span>
				</label>
			</div>
		{/if}

		<div class="flex flex-wrap gap-2">
			<button
				type="button"
				class="ws-btn ws-btn-primary"
				onclick={() => onchange(setBand(profile, z0, z1, bandHeight))}
			>
				{$t('Apply band')}
			</button>
			{#if onadaptive}
				<button
					type="button"
					disabled={busy}
					class="ws-btn ws-btn-secondary"
					onclick={() => run(() => onadaptive(quality))}
				>
					{$t('Adaptive')}
				</button>
			{/if}
			<button
				type="button"
				disabled={busy}
				class="ws-btn ws-btn-secondary"
				onclick={() =>
					onsmooth ? run(() => onsmooth(radius, keepMin)) : onchange(smooth(profile, 0.5))}
			>
				{$t('Smooth||verb')}
			</button>
			<button
				type="button"
				class="ws-btn ws-btn-secondary"
				onclick={() => onchange(uniformProfile(zMax, baseHeight))}
			>
				{$t('Reset')}
			</button>
		</div>
	</div>
</div>
