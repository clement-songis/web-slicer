<script lang="ts">
	// Barre d'outils de plateau horizontale (T106) : miroir de la barre principale
	// OrcaSlicer (`GLCanvas3D::_init_main_toolbar` + vue d'assemblage), tracée dans
	// `traceability-map.json#toolbars`. Présentational : émet l'identifiant d'action
	// au parent (la page orchestre ajout/arrange/orient/split/instances/couches).
	import { MAIN_TOOLBAR, ASSEMBLE_TOOLBAR, type ToolbarButton } from './menus';
	import { t } from '$lib/i18n';
	import { shortcutKeyFor, withShortcut } from '$lib/editor';
	import IconAdd from '~icons/lucide/plus';
	import IconAddPlate from '~icons/lucide/copy-plus';
	import IconOrient from '~icons/lucide/compass';
	import IconArrange from '~icons/lucide/layout-grid';
	import IconMore from '~icons/lucide/circle-plus';
	import IconFewer from '~icons/lucide/circle-minus';
	import IconSplitObjects from '~icons/lucide/split';
	import IconSplitVolumes from '~icons/lucide/split-square-horizontal';
	import IconLayers from '~icons/lucide/layers';
	import IconAssembly from '~icons/lucide/boxes';

	interface Props {
		/** Identifiants d'items en état actif (bascules : layersediting, assembly_view). */
		active?: Set<string>;
		/** Objet sélectionné présent → active les items dépendant d'une sélection. */
		hasSelection?: boolean;
		/** Scène non vide → active arrange/orient. */
		hasObjects?: boolean;
		onaction: (id: string) => void;
	}

	let {
		active = new Set<string>(),
		hasSelection = false,
		hasObjects = false,
		onaction
	}: Props = $props();

	// Items affichés : barre principale + vue d'assemblage (le séparateur d'assemblage
	// n'est pas un bouton). `sep` insère un séparateur avant l'item.
	type Item = ToolbarButton & { sep?: boolean };
	// Familles séparées : ajout | disposition | instances | découpe | couches | assemblage.
	const SEP_BEFORE = new Set(['orient', 'more', 'splitobjects', 'layersediting']);
	const ITEMS: Item[] = [
		...MAIN_TOOLBAR.map((item) => ({ ...item, sep: SEP_BEFORE.has(item.id) })),
		{ ...ASSEMBLE_TOOLBAR[0], sep: true }
	];

	const ICONS: Record<string, typeof IconAdd> = {
		add: IconAdd,
		addplate: IconAddPlate,
		orient: IconOrient,
		arrange: IconArrange,
		more: IconMore,
		fewer: IconFewer,
		splitobjects: IconSplitObjects,
		splitvolumes: IconSplitVolumes,
		layersediting: IconLayers,
		assembly_view: IconAssembly
	};

	// Items désactivés sans sélection (instances/split) ou sans objet (arrange/orient).
	const NEEDS_SELECTION = new Set(['more', 'fewer', 'splitobjects', 'splitvolumes']);
	const NEEDS_OBJECTS = new Set(['arrange', 'orient']);
	// Noms accessibles distincts : « Add plate » de la barre ne doit pas reprendre le
	// nom du bouton « + » de la barre de plateaux (« Add a plate » → « Ajouter un
	// plateau » en français) — clé à contexte, affichée « Add plate » en anglais.
	const I18N_KEY: Record<string, string> = { addplate: 'Add plate||toolbar' };
	const labelOf = (item: ToolbarButton) => I18N_KEY[item.id] ?? item.label;

	function disabled(id: string): boolean {
		if (NEEDS_SELECTION.has(id)) return !hasSelection;
		if (NEEDS_OBJECTS.has(id)) return !hasObjects;
		return false;
	}
</script>

<div
	class="ws-floating flex items-center gap-0.5 p-1"
	role="toolbar"
	aria-label={$t('Plate toolbar')}
>
	{#each ITEMS as item (item.id)}
		{@const Icon = ICONS[item.id]}
		{#if item.sep}
			<div class="mx-1 h-6 w-px bg-border" aria-hidden="true"></div>
		{/if}
		<button
			type="button"
			class="ws-tool ws-tool-sm {active.has(item.id) ? 'ws-tool-on' : ''}"
			aria-pressed={active.has(item.id)}
			disabled={disabled(item.id)}
			data-testid={`toolbar-${item.id}`}
			aria-label={$t(labelOf(item))}
			aria-keyshortcuts={shortcutKeyFor(item.id) ?? undefined}
			data-tip={withShortcut($t(labelOf(item)), shortcutKeyFor(item.id))}
			data-tip-side="bottom"
			onclick={() => onaction(item.id)}
		>
			{#if Icon}<Icon class="h-5 w-5" aria-hidden="true" />{/if}
		</button>
	{/each}
</div>
