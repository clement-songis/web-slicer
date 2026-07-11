<script lang="ts">
	// Panneau numérique de transformation (T052) : saisie précise de la position
	// (mm), rotation (°) et échelle de l'objet sélectionné, plus « poser à plat ».
	// Émet la nouvelle transformation ; le parent applique et historise.
	import { clampScale, normalizeAngle, type Transform } from '../transform';

	interface Props {
		transform: Transform;
		onchange: (t: Transform) => void;
		/** Pose à plat (le parent calcule la rotation depuis la facette choisie). */
		onlayflat?: () => void;
		/** Actif seulement si une facette est sélectionnée. */
		canLayFlat?: boolean;
	}

	let { transform, onchange, onlayflat, canLayFlat = false }: Props = $props();

	type Field = 'position' | 'rotation' | 'scale';
	const rows: { field: Field; label: string }[] = [
		{ field: 'position', label: 'Position (mm)' },
		{ field: 'rotation', label: 'Rotation (°)' },
		{ field: 'scale', label: 'Échelle' }
	];
	const axes = ['X', 'Y', 'Z'];

	function setAxis(field: Field, index: number, raw: string) {
		let n = Number(raw);
		if (!Number.isFinite(n)) return;
		if (field === 'rotation') n = normalizeAngle(n);
		if (field === 'scale') n = clampScale(n);
		const next: Transform = {
			position: [...transform.position],
			rotation: [...transform.rotation],
			scale: [...transform.scale]
		};
		next[field][index] = n;
		onchange(next);
	}
</script>

<div class="flex flex-col gap-2 text-sm">
	{#each rows as row (row.field)}
		<div class="grid grid-cols-[6rem_repeat(3,1fr)] items-center gap-2">
			<span class="text-content-muted">{row.label}</span>
			{#each axes as axis, i (axis)}
				<label class="flex items-center gap-1">
					<span class="text-content-subtle">{axis}</span>
					<input
						type="number"
						step={row.field === 'scale' ? 0.01 : 1}
						value={transform[row.field][i]}
						oninput={(e) => setAxis(row.field, i, e.currentTarget.value)}
						class="w-full rounded border border-border-strong bg-surface-raised px-1 py-0.5 text-right"
						aria-label={`${row.label} ${axis}`}
					/>
				</label>
			{/each}
		</div>
	{/each}

	<button
		type="button"
		class="mt-1 self-start rounded border border-border-strong bg-surface-sunken px-3 py-1 text-content hover:bg-overlay disabled:opacity-40"
		disabled={!canLayFlat}
		onclick={() => onlayflat?.()}
	>
		Poser à plat
	</button>
</div>
