<script lang="ts">
	// Navigateur 3D (Orca : cube ViewManipulate en bas à gauche de la vue) :
	// cube orienté comme la caméra ; un clic sur une face choisit la vue
	// correspondante, le bouton central revient à la vue par défaut (Ctrl+0).
	import type { NamedView } from './camera';
	import { t } from '$lib/i18n';
	import { CUBE_FACES, cubeTransform, faceTransform, facingFace } from './view-cube';

	interface Props {
		/** Direction de vue de la caméra (repère monde, Z vertical). */
		forward: [number, number, number];
		onselect: (view: NamedView) => void;
	}

	let { forward, onselect }: Props = $props();

	const SIZE = 56;
	const half = SIZE / 2;
	const transform = $derived(cubeTransform(forward));
	const facing = $derived(facingFace(forward).view);
</script>

<div
	class="pointer-events-auto flex items-end gap-2 select-none"
	data-testid="view-cube"
	data-facing={facing}
	role="group"
	aria-label={$t('3D navigator')}
>
	<!-- Marge : la projection en perspective déborde de la boîte du cube. -->
	<div
		class="relative m-2"
		style:width="{SIZE}px"
		style:height="{SIZE}px"
		style:perspective="400px"
	>
		<div
			class="absolute inset-0"
			style:transform-style="preserve-3d"
			style:transform
			data-testid="view-cube-body"
		>
			{#each CUBE_FACES as face (face.view)}
				<button
					type="button"
					class="ws-view-cube-face absolute inset-0 flex items-center justify-center rounded-sm border border-border-strong text-2xs font-semibold text-content {facing ===
					face.view
						? 'bg-surface-raised'
						: 'bg-surface-sunken'} hover:bg-primary hover:text-primary-content"
					style:transform={faceTransform(face, half)}
					style:backface-visibility="hidden"
					data-testid="view-cube-face"
					data-view={face.view}
					aria-label={$t('View {face}', { face: $t(face.label) })}
					onclick={() => onselect(face.view)}>{$t(face.label)}</button
				>
			{/each}
		</div>
	</div>
	<button
		type="button"
		class="ws-chip px-1.5 py-0 text-2xs"
		data-testid="view-cube-home"
		data-tip={$t('Default view (Ctrl+0)')}
		data-tip-side="right"
		onclick={() => onselect('default')}>Iso</button
	>
</div>
