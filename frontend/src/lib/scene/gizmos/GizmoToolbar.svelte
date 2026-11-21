<script lang="ts">
	// Barre de sélection du mode de gizmo (T052) : déplacer / tourner / échelle.
	import { GIZMO_MODE_LABELS, type GizmoMode } from './types';
	import { t } from '$lib/i18n';

	interface Props {
		mode: GizmoMode;
		onmode: (mode: GizmoMode) => void;
	}

	let { mode, onmode }: Props = $props();

	const modes: GizmoMode[] = ['translate', 'rotate', 'scale'];
</script>

<div class="ws-seg" role="group">
	{#each modes as m (m)}
		<button
			type="button"
			class="ws-seg-item {mode === m ? 'ws-seg-on' : ''}"
			aria-pressed={mode === m}
			onclick={() => onmode(m)}
		>
			{$t(GIZMO_MODE_LABELS[m])}
		</button>
	{/each}
</div>
