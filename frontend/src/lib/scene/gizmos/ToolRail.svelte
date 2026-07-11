<script lang="ts">
	// Rail d'outils vertical (T103) : expose les 16 gizmos OrcaSlicer
	// (`traceability-map.json#gizmos`) sous forme de boutons icônes. Sélectionne
	// l'outil actif ; les panneaux/comportements sont câblés par T103 (transform),
	// T104 (outils maillage) et T105 (peinture). Présentational : l'état vit dans
	// `lib/editor/tools.ts`.
	import { TOOL_ORDER, type EditorTool } from '$lib/editor';
	import { t } from '$lib/i18n';
	import IconMove from '~icons/lucide/move';
	import IconRotate from '~icons/lucide/rotate-3d';
	import IconScale from '~icons/lucide/scaling';
	import IconFlatten from '~icons/lucide/arrow-down-to-line';
	import IconCut from '~icons/lucide/scissors';
	import IconBoolean from '~icons/lucide/combine';
	import IconSupport from '~icons/lucide/brush';
	import IconSeam from '~icons/lucide/spline';
	import IconFuzzy from '~icons/lucide/waves';
	import IconColor from '~icons/lucide/palette';
	import IconText from '~icons/lucide/type';
	import IconSvg from '~icons/lucide/shapes';
	import IconMeasure from '~icons/lucide/ruler';
	import IconAssembly from '~icons/lucide/boxes';
	import IconSimplify from '~icons/lucide/minimize-2';
	import IconBrim from '~icons/lucide/anchor';

	interface Props {
		/** Outil actif (ou null). */
		active: EditorTool | null;
		/** Sélection d'un outil. */
		onselect: (tool: EditorTool) => void;
	}

	let { active, onselect }: Props = $props();

	// Métadonnées d'affichage : libellé de parité (clé i18n) + icône. `sep` insère
	// un séparateur avant l'outil (transformation | outils maillage | peinture).
	type Meta = { label: string; icon: typeof IconMove; sep?: boolean };
	const META: Record<EditorTool, Meta> = {
		move: { label: 'Move', icon: IconMove },
		rotate: { label: 'Rotate', icon: IconRotate },
		scale: { label: 'Scale', icon: IconScale },
		flatten: { label: 'Flatten', icon: IconFlatten, sep: true },
		cut: { label: 'Cut', icon: IconCut },
		boolean: { label: 'Mesh Boolean', icon: IconBoolean },
		'support-paint': { label: 'Supports Painting', icon: IconSupport, sep: true },
		'seam-paint': { label: 'Seam painting', icon: IconSeam },
		'fuzzy-paint': { label: 'Fuzzy Skin painting', icon: IconFuzzy },
		'mm-paint': { label: 'Color Painting', icon: IconColor },
		emboss: { label: 'Text shape', icon: IconText, sep: true },
		svg: { label: 'SVG shape', icon: IconSvg },
		measure: { label: 'Measure', icon: IconMeasure },
		assembly: { label: 'Assembly View', icon: IconAssembly },
		simplify: { label: 'Simplify model', icon: IconSimplify },
		'brim-ears': { label: 'Auto-Brim', icon: IconBrim }
	};
</script>

<div class="flex flex-col items-center gap-0.5" role="toolbar" aria-label="Outils">
	{#each TOOL_ORDER as tool (tool)}
		{@const meta = META[tool]}
		{@const Icon = meta.icon}
		{#if meta.sep}
			<div class="my-1 h-px w-6 bg-border"></div>
		{/if}
		<button
			type="button"
			class="flex h-9 w-9 items-center justify-center rounded {active === tool
				? 'bg-primary text-primary-content'
				: 'text-content-muted hover:bg-overlay hover:text-content'}"
			aria-pressed={active === tool}
			title={$t(meta.label)}
			onclick={() => onselect(tool)}
		>
			<Icon class="h-5 w-5" />
		</button>
	{/each}
</div>
