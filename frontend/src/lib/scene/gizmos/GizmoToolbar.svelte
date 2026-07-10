<script lang="ts">
	// Barre de sélection du mode de gizmo (T052) : déplacer / tourner / échelle.
	import { GIZMO_MODE_LABELS, type GizmoMode } from './types';

	interface Props {
		mode: GizmoMode;
		onmode: (mode: GizmoMode) => void;
	}

	let { mode, onmode }: Props = $props();

	const modes: GizmoMode[] = ['translate', 'rotate', 'scale'];
</script>

<div class="inline-flex overflow-hidden rounded border border-slate-600" role="group">
	{#each modes as m (m)}
		<button
			type="button"
			class="px-3 py-1 text-sm {mode === m
				? 'bg-slate-600 text-white'
				: 'bg-slate-800 text-slate-300 hover:bg-slate-700'}"
			aria-pressed={mode === m}
			onclick={() => onmode(m)}
		>
			{GIZMO_MODE_LABELS[m]}
		</button>
	{/each}
</div>
