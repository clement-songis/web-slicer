<script lang="ts">
	// Rail d'outils vertical (T103) : expose les 16 gizmos OrcaSlicer
	// (`traceability-map.json#gizmos`) sous forme de boutons icônes. Sélectionne
	// l'outil actif ; les panneaux/comportements sont câblés par T103 (transform),
	// T104 (outils maillage) et T105 (peinture). Présentational : l'état vit dans
	// `lib/editor/tools.ts`.
	import { TOOL_ORDER, shortcutKeyFor, withShortcut, type EditorTool } from '$lib/editor';
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

	// Métadonnées d'affichage : libellé de parité (clé i18n), icône et famille
	// (transformer | maillage | peinture | ajouter une forme | outils).
	type Family = 'transform' | 'mesh' | 'paint' | 'add' | 'tools';
	type Meta = { label: string; icon: typeof IconMove; family: Family };
	const META: Record<EditorTool, Meta> = {
		move: { label: 'Move', icon: IconMove, family: 'transform' },
		rotate: { label: 'Rotate', icon: IconRotate, family: 'transform' },
		scale: { label: 'Scale', icon: IconScale, family: 'transform' },
		flatten: { label: 'Flatten', icon: IconFlatten, family: 'mesh' },
		cut: { label: 'Cut', icon: IconCut, family: 'mesh' },
		boolean: { label: 'Mesh Boolean', icon: IconBoolean, family: 'mesh' },
		'support-paint': { label: 'Supports Painting', icon: IconSupport, family: 'paint' },
		'seam-paint': { label: 'Seam painting', icon: IconSeam, family: 'paint' },
		'fuzzy-paint': { label: 'Fuzzy Skin painting', icon: IconFuzzy, family: 'paint' },
		'mm-paint': { label: 'Color Painting', icon: IconColor, family: 'paint' },
		emboss: { label: 'Text shape', icon: IconText, family: 'add' },
		svg: { label: 'SVG shape', icon: IconSvg, family: 'add' },
		measure: { label: 'Measure', icon: IconMeasure, family: 'tools' },
		assembly: { label: 'Assembly View', icon: IconAssembly, family: 'tools' },
		simplify: { label: 'Simplify model', icon: IconSimplify, family: 'tools' },
		'brim-ears': { label: 'Auto-Brim', icon: IconBrim, family: 'tools' }
	};

	// Familles du rail (comme les groupes de la barre de gizmos d'Orca) : chaque
	// famille est un groupe nommé, séparé visuellement des voisins.
	const FAMILY_LABELS: Record<Family, string> = {
		transform: 'Transformer',
		mesh: 'Maillage',
		paint: 'Peinture',
		add: 'Ajouter une forme',
		tools: 'Outils'
	};
	const groups = $derived.by(() => {
		const out: { family: Family; tools: EditorTool[] }[] = [];
		for (const tool of TOOL_ORDER) {
			const family = META[tool].family;
			const last = out.at(-1);
			if (last && last.family === family) last.tools.push(tool);
			else out.push({ family, tools: [tool] });
		}
		return out;
	});
</script>

<div
	class="flex flex-col items-center gap-1"
	role="toolbar"
	aria-label="Outils"
	aria-orientation="vertical"
>
	{#each groups as group, gi (group.family)}
		{#if gi > 0}
			<div class="h-px w-6 bg-border" aria-hidden="true"></div>
		{/if}
		<div
			class="flex flex-col items-center gap-0.5"
			role="group"
			aria-label={FAMILY_LABELS[group.family]}
		>
			{#each group.tools as tool (tool)}
				{@const meta = META[tool]}
				{@const Icon = meta.icon}
				{@const key = shortcutKeyFor(tool)}
				<!-- Info-bulle stylée « Nom (Touche) » comme Orca (TooltipLayer) ;
				     nom accessible = nom seul. -->
				<button
					type="button"
					class="ws-tool {active === tool ? 'ws-tool-on' : ''}"
					aria-pressed={active === tool}
					aria-label={$t(meta.label)}
					aria-keyshortcuts={key ?? undefined}
					data-testid={`tool-${tool}`}
					data-tip={withShortcut($t(meta.label), key)}
					data-tip-side="left"
					onclick={() => onselect(tool)}
				>
					<Icon class="h-5 w-5" aria-hidden="true" />
				</button>
			{/each}
		</div>
	{/each}
</div>
