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

	interface Props {
		profile: LayerBand[];
		zMax: number;
		/** Hauteur de couche par défaut (réinitialisation). */
		baseHeight?: number;
		onchange: (profile: LayerBand[]) => void;
	}

	let { profile, zMax, baseHeight = 0.2, onchange }: Props = $props();

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
	<svg viewBox="0 0 100 100" class="h-40 w-16 rounded border border-slate-700 bg-slate-900">
		<polyline points={samples} fill="none" stroke="#38bdf8" stroke-width="1.5" />
	</svg>

	<div class="flex flex-1 flex-col gap-2">
		<div class="grid grid-cols-3 gap-1">
			<label class="flex flex-col">
				<span class="text-slate-400">Z de</span>
				<input
					type="number"
					min={0}
					max={zMax}
					step={0.1}
					bind:value={z0}
					class="rounded border border-slate-600 bg-slate-900 px-1"
				/>
			</label>
			<label class="flex flex-col">
				<span class="text-slate-400">Z à</span>
				<input
					type="number"
					min={0}
					max={zMax}
					step={0.1}
					bind:value={z1}
					class="rounded border border-slate-600 bg-slate-900 px-1"
				/>
			</label>
			<label class="flex flex-col">
				<span class="text-slate-400">Hauteur</span>
				<input
					type="number"
					min={DEFAULT_MIN_HEIGHT}
					max={DEFAULT_MAX_HEIGHT}
					step={0.01}
					bind:value={bandHeight}
					class="rounded border border-slate-600 bg-slate-900 px-1"
				/>
			</label>
		</div>

		<div class="flex flex-wrap gap-2">
			<button
				type="button"
				class="rounded bg-sky-600 px-3 py-1 text-white hover:bg-sky-500"
				onclick={() => onchange(setBand(profile, z0, z1, bandHeight))}>Appliquer la bande</button
			>
			<button
				type="button"
				class="rounded border border-slate-600 px-3 py-1 hover:bg-slate-700"
				onclick={() => onchange(smooth(profile, 0.5))}>Lisser</button
			>
			<button
				type="button"
				class="rounded border border-slate-600 px-3 py-1 hover:bg-slate-700"
				onclick={() => onchange(uniformProfile(zMax, baseHeight))}>Réinitialiser</button
			>
		</div>
	</div>
</div>
