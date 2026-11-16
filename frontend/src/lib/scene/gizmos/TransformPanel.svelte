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
		<!-- Libellé + 3 champs ; l'axe est un préfixe dans le champ, la valeur
		     dispose de toute la largeur (« -123.45 » sans troncature). -->
		<div class="grid grid-cols-[4rem_repeat(3,minmax(0,1fr))] items-center gap-1.5">
			<span class="text-xs leading-tight text-content-muted">{row.label}</span>
			{#each axes as axis, i (axis)}
				<label class="relative flex min-w-0 items-center">
					<span
						class="pointer-events-none absolute left-1.5 text-2xs font-semibold text-content-subtle"
						aria-hidden="true">{axis}</span
					>
					<input
						type="number"
						step={row.field === 'scale' ? 0.01 : 1}
						value={transform[row.field][i]}
						oninput={(e) => setAxis(row.field, i, e.currentTarget.value)}
						class="ws-field ws-field-sm w-full min-w-0 pr-1.5 pl-4 text-right"
						aria-label={`${row.label} ${axis}`}
					/>
				</label>
			{/each}
		</div>
	{/each}

	<button
		type="button"
		class="ws-btn ws-btn-secondary mt-1 self-start"
		disabled={!canLayFlat}
		onclick={() => onlayflat?.()}
	>
		Poser à plat
	</button>
</div>
