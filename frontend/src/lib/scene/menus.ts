// Menus contextuels, barres d'outils et raccourcis clavier du plateau et de la
// liste d'objets (T061, Annexe B §B.3–B.6). Vocabulaire d'interface tracé dans
// `specs/001-orcaslicer-web-parity/traceability-map.json` et vérifié par
// `audit/check_traceability.py` (T062). Les libellés anglais sont les clés de
// parité (issus d'OrcaSlicer) ; l'affichage localisé est une couche i18n.

/** Bouton de barre d'outils 3D (B.4). */
export interface ToolbarButton {
	id: string;
	label: string;
}

/** Barre d'outils principale (`GLCanvas3D::_init_main_toolbar`). */
export const MAIN_TOOLBAR: ToolbarButton[] = [
	{ id: 'add', label: 'Add' },
	{ id: 'addplate', label: 'Add plate' },
	{ id: 'orient', label: 'Auto orient' },
	{ id: 'arrange', label: 'Arrange' },
	{ id: 'more', label: 'Add instance' },
	{ id: 'fewer', label: 'Remove instance' },
	{ id: 'splitobjects', label: 'Split to objects' },
	{ id: 'splitvolumes', label: 'Split to parts' },
	{ id: 'layersediting', label: 'Variable layer height' }
];

/** Barre d'outils d'assemblage + séparateur. */
export const ASSEMBLE_TOOLBAR: ToolbarButton[] = [
	{ id: 'assembly_view', label: 'Assembly view' },
	{ id: 'start_seperator', label: 'Separator' }
];

/** Entrée de menu contextuel du plateau (B.3). */
export interface ContextMenuItem {
	label: string;
}

/**
 * Libellés du menu contextuel du plateau (Annexe B §B.3, dédupliqués) —
 * recalibrés sur l'inventaire canonique d'Orca main (aefb6b3d, flip 1e4abcd) :
 * casse (« Fix Model », « To Objects »), sous-menus titrés (« Mirror: »,
 * « Split: ») et nouveaux libellés main. « Delete Plate » : action plateau
 * toujours câblée web (plate.delete), absente de l'inventaire main — gardée.
 */
export const PLATER_MENU: ContextMenuItem[] = [
	'HideShow',
	'Delete',
	'Load...',
	'Cube',
	'Cylinder',
	'Sphere',
	'Cone',
	'Disc',
	'Torus',
	'Height Range Modifier',
	'Set as An Individual Object',
	'Fill bed with copies',
	'Printable',
	'Rename',
	'Fix Model',
	'Export as one STL…',
	'Export as STLs…',
	'Export as one DRC…',
	'Export as DRCs…',
	'Reload from disk',
	'Replace 3D file…',
	'Replace all with 3D files…',
	'Scale to build volume',
	"Flush into objects' infill",
	'Flush into this object',
	"Flush into objects' support",
	'Assemble',
	'Mirror:',
	'Along X Axis',
	'Along Y Axis',
	'Along Z Axis',
	'Split:',
	'To Objects',
	'To Parts',
	'Mesh boolean',
	'Add Models',
	'Show Labels',
	'Auto orientation',
	'Edit Painting',
	'Edit',
	'Decompose Color',
	'Select All',
	'Select All Plates',
	'Delete All',
	'Arrange',
	'Auto Rotate',
	'Fix All',
	'Fix All Plates',
	'Reload All',
	'Delete Plate',
	'Add instance',
	'Remove instance',
	'Set number of instances…',
	'Fill bed with instances…',
	'Clone',
	'Simplify Model',
	'Subdivision mesh',
	'Center',
	'Drop'
].map((label) => ({ label }));

/** Entrée du menu contextuel objet (T112) : libellé de parité + identifiant d'action. */
export interface ObjectMenuItem {
	label: string;
	action: string;
	/** Précision affichée en second (rôle de l'entrée). */
	hint?: string;
	/** Sous-menu (déplié dans le menu, ex. « Modèles pratiques »). */
	items?: ObjectMenuItem[];
	/**
	 * Sous-menu **ouvert** (phase 25) : rendu en section titrée, ses entrées
	 * toujours visibles (menu en colonnes). Les specs gelés cliquent les actions
	 * directement après le clic droit : un sous-menu replié les rendrait
	 * inatteignables, la section garde donc chaque action à un clic.
	 */
	section?: true;
	/**
	 * Audit 43 M1-a — entrée qui est **à la fois** une action et un sous-menu
	 * (« Ajouter un modificateur » ▸ Charger…, Cube…). Le clic sur le libellé
	 * garde l'action d'origine (pavé, specs gelés) ; la flèche déplie les formes
	 * d'Orca (`append_submenu_add_generic`).
	 */
	split?: true;
}

/**
 * Formes du sous-menu « Ajouter une pièce / un volume négatif / un modificateur /
 * un bloqueur / un forceur » (MenuFactory::append_submenu_add_generic,
 * GUI_Factories.cpp:541) : Charger…, Cube, Cylindre, Sphère, Cône, Disque, Tore,
 * Texte, SVG — même ordre qu'Orca.
 */
export const VOLUME_SHAPES = [
	{ label: 'Load...', shape: 'load' },
	{ label: 'Cube', shape: 'cube' },
	{ label: 'Cylinder', shape: 'cylinder' },
	{ label: 'Sphere', shape: 'sphere' },
	{ label: 'Cone', shape: 'cone' },
	{ label: 'Disc', shape: 'disc' },
	{ label: 'Torus', shape: 'torus' },
	{ label: 'Text', shape: 'text' },
	{ label: 'SVG', shape: 'svg' }
] as const;

export type VolumeShape = (typeof VOLUME_SHAPES)[number]['shape'];

/** Entrées du sous-menu de formes d'une action d'ajout de volume. */
export function volumeShapeItems(action: string): ObjectMenuItem[] {
	return VOLUME_SHAPES.map((s) => ({ label: s.label, action: `${action}:${s.shape}` }));
}

/** `object.addModifier:cylinder` → { base: 'object.addModifier', shape: 'cylinder' }. */
export function parseVolumeShapeAction(
	action: string
): { base: string; shape: VolumeShape } | null {
	const i = action.lastIndexOf(':');
	if (i < 0) return null;
	const shape = action.slice(i + 1);
	const found = VOLUME_SHAPES.find((s) => s.shape === shape);
	return found ? { base: action.slice(0, i), shape: found.shape } : null;
}

/** Entrée d'ajout de volume avec son sous-menu de formes. */
function addVolumeItem(label: string, action: string): ObjectMenuItem {
	return { label, action, split: true, items: volumeShapeItems(action) };
}

/** Entrée du menu contextuel objet, avec séparateurs. */
export type ObjectMenuEntry = ObjectMenuItem | 'separator';

/**
 * Actions objet du menu contextuel (sous-ensemble objet de `context_menu`, T112).
 * Libellés = clés de parité (tracés dans `traceability-map.json#context_menu`).
 */
export const OBJECT_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	// Phase 25 : les 43 entrées à plat (plus hautes que l'écran) sont rangées en
	// sous-menus ouverts, à la manière des sous-menus d'Orca (`create_object_menu` :
	// Add part/modifier…, Mirror, Split, Export) — aucune action retirée.
	{
		label: 'Object',
		action: 'section.object',
		section: true,
		items: [
			{ label: 'HideShow', action: 'object.hideShow' },
			{ label: 'Rename', action: 'object.rename' },
			{ label: 'Edit', action: 'object.edit' },
			{ label: 'Add settings', action: 'object.addSettings' },
			{ label: 'Delete', action: 'object.delete' }
		]
	},
	{
		label: 'Instances',
		action: 'section.instances',
		section: true,
		items: [
			{ label: 'Clone', action: 'object.clone' },
			{ label: 'Add instance', action: 'object.addInstance' },
			{ label: 'Remove instance', action: 'object.removeInstance' },
			{ label: 'Set number of instances…', action: 'object.setInstances' },
			{ label: 'Set as an individual object', action: 'object.individual' },
			{ label: 'Fill bed with copies', action: 'object.fillBed' },
			{ label: 'Fill bed with instances…', action: 'object.fillBedInstances' }
		]
	},
	{
		label: 'Placement',
		action: 'section.placement',
		section: true,
		items: [
			{ label: 'Auto orientation', action: 'object.orient' },
			{ label: 'Center', action: 'object.center' },
			{ label: 'Drop', action: 'object.drop' },
			{ label: 'Scale to build volume', action: 'object.scaleToBed' }
		]
	},
	// Sous-menu Mirror d'Orca (`append_menu_items_mirror`). Le rappel « Miroir »
	// (`hint`) reste dans les données (entrées hors section, tests) ; la section
	// porte déjà ce titre, ContextMenu ne le répète pas.
	{
		label: 'Mirror',
		action: 'section.mirror',
		section: true,
		items: [
			{ label: 'Along X axis', action: 'object.mirrorX', hint: 'Mirror' },
			{ label: 'Along Y axis', action: 'object.mirrorY', hint: 'Mirror' },
			{ label: 'Along Z axis', action: 'object.mirrorZ', hint: 'Mirror' }
		]
	},
	{
		label: 'Parts and modifiers',
		action: 'section.parts',
		section: true,
		items: [
			addVolumeItem('Add part', 'object.addPart'),
			addVolumeItem('Add negative volume', 'object.addNegativeVolume'),
			addVolumeItem('Add modifier', 'object.addModifier'),
			{ label: 'Height range Modifier', action: 'object.addHeightRange' },
			addVolumeItem('Add support blocker', 'object.addSupportBlocker'),
			addVolumeItem('Add support enforcer', 'object.addSupportEnforcer')
		]
	},
	{
		label: 'Mesh',
		action: 'section.mesh',
		section: true,
		items: [
			{ label: 'Fix model', action: 'object.fix' },
			{ label: 'Simplify Model', action: 'object.simplify' },
			{ label: 'Subdivision mesh(Lost color)', action: 'object.smooth' },
			{ label: 'Mesh boolean', action: 'object.meshBoolean' },
			{ label: 'Reload from disk', action: 'object.reload' },
			{ label: 'Replace 3D file…', action: 'object.replace' },
			{ label: 'Replace all with 3D files…', action: 'object.replaceAll' }
		]
	},
	// Sous-menu Split d'Orca (`append_menu_item_split` : To objects / To parts)
	// + assemblage.
	{
		label: 'Split and assemble',
		action: 'section.split',
		section: true,
		items: [
			{ label: 'Split', action: 'object.split' },
			{ label: 'To objects', action: 'object.toObjects' },
			{ label: 'To parts', action: 'object.toParts' },
			{ label: 'Assemble', action: 'object.assemble' },
			{ label: 'Assemble (single part)', action: 'object.assembleSingle' }
		]
	},
	{
		label: 'Export',
		action: 'section.export',
		section: true,
		items: [
			{ label: 'Export as one STL…', action: 'object.exportStl' },
			{ label: 'Export as STLs…', action: 'object.exportStls' },
			{ label: 'Export as one DRC…', action: 'object.exportDrc' },
			{ label: 'Export as DRCs…', action: 'object.exportDrcs' }
		]
	}
];

/** Actions feuilles d'un menu (sections et sous-menus dépliés), dans l'ordre. */
export function menuLeaves(entries: readonly ObjectMenuEntry[]): ObjectMenuItem[] {
	return entries.flatMap((e) =>
		e === 'separator' ? [] : e.items && !e.split ? menuLeaves(e.items) : [e]
	);
}

/**
 * Menu d'une pièce (ligne enfant d'un objet multi-pièces) : `create_part_menu`
 * (« Split » = `split_volume`) et `create_bbl_part_menu` (sous-menu Split :
 * « To objects » = `split_object` de tout l'objet parent, « To parts » =
 * `split_volume` de la pièce).
 */
export const PART_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	{ label: 'Rename', action: 'object.rename' },
	{ label: 'Fix model', action: 'object.fix' },
	{ label: 'Subdivision mesh(Lost color)', action: 'object.smooth' },
	'separator',
	{ label: 'Split', action: 'part.split' },
	{ label: 'To objects', action: 'part.toObjects' },
	{ label: 'To parts', action: 'part.toParts' },
	'separator',
	{ label: 'Delete', action: 'object.delete' }
];

/**
 * Menu d'ajout de la scène (clic droit plateau vide + bouton `add`, T113).
 * Libellés = clés de parité (`context_menu`) : Load + primitives.
 */
export const SCENE_ADD_ITEMS: ObjectMenuEntry[] = [
	{ label: 'Load...', action: 'add.load' },
	'separator',
	{ label: 'Cube', action: 'add.cube' },
	{ label: 'Cylinder', action: 'add.cylinder' },
	{ label: 'Sphere', action: 'add.sphere' },
	{ label: 'Cone', action: 'add.cone' },
	{ label: 'Disc', action: 'add.disc' },
	{ label: 'Torus', action: 'add.torus' }
];

/** Menu contextuel d'un onglet de plateau (`MenuFactory::create_plate_menu`). */
export const PLATE_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	{ label: 'Select All', action: 'plate.selectAll' },
	{ label: 'Select All Plates', action: 'plate.selectAllPlates' },
	{ label: 'Delete All', action: 'plate.deleteAll' },
	'separator',
	{ label: 'Arrange', action: 'plate.arrange' },
	{ label: 'Auto Rotate', action: 'plate.autoRotate' },
	{ label: 'Reload All', action: 'plate.reloadAll' },
	'separator',
	{ label: 'Delete Plate', action: 'plate.delete' },
	'separator',
	{ label: 'Add Models', action: 'plate.addModels' }
];

/** Menu contextuel d'un slot filament (`create_filament_action_menu`). */
export const FILAMENT_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	// « Edit » du menu filament = verbe (Orca : « Éditer »), distinct du menu
	// principal « Edit » (« Édition ») : clé à contexte, affichée « Edit » en anglais.
	{ label: 'Edit||filament', action: 'filament.edit' },
	{ label: 'Delete', action: 'filament.delete' }
];

/**
 * Clic droit sur le plateau vide (`MenuFactory::create_default_menu`) : menu
 * d'ajout + bascule « Show Labels ».
 */
export const SCENE_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	...SCENE_ADD_ITEMS,
	'separator',
	// `create_default_menu` : « Add Models » ouvre le sélecteur de fichiers.
	{ label: 'Add Models', action: 'add.load' },
	{ label: 'Show Labels', action: 'view.labels' }
];

/** Raccourci clavier : combinaison (verbatim Annexe B) → action. */
export interface Shortcut {
	keys: string;
	action: string;
}

/**
 * Raccourcis du plateau — groupe « Placement » de l'annexe B §B.6 (Orca main
 * aefb6b3d : l'ancien groupe « Plater » y est éclaté en Placement / Editing /
 * Gizmos / Camera / Application — 13 groupes × 98 raccourcis). Le tableau
 * garde l'union des raccourcis plateau câblés web : formes canoniques main
 * (« Up », « PageUp »…) et entrées souris/flèches 2.4.1 toujours vraies.
 */
export const PLATER_SHORTCUTS: Shortcut[] = [
	{ keys: 'Mouse wheel', action: 'Zoom' },
	{ keys: 'A', action: 'Arrange all objects' },
	{ keys: 'Shift+A', action: 'Arrange objects on selected plates' },
	{ keys: 'Q', action: 'Auto orient selection or all' },
	{ keys: 'Shift+Q', action: 'Auto orient all on active plate' },
	{ keys: 'Shift+Tab', action: 'Collapse/expand sidebar' },
	{ keys: 'Ctrl+Any arrow', action: 'Movement in camera space' },
	{ keys: 'Alt+Left mouse button', action: 'Select a part' },
	{ keys: 'Ctrl+Left mouse button', action: 'Select multiple objects' },
	{ keys: 'Shift+Left mouse button', action: 'Select by rectangle' },
	{ keys: 'Up', action: 'Move selection 10mm in positive Y direction' },
	{ keys: 'Down', action: 'Move selection 10mm in negative Y direction' },
	{ keys: 'Left', action: 'Move selection 10mm in negative X direction' },
	{ keys: 'Right', action: 'Move selection 10mm in positive X direction' },
	{ keys: 'PageUp', action: 'Rotate selection 45 degrees counterclockwise' },
	{ keys: 'PageDown', action: 'Rotate selection 45 degrees clockwise' },
	{ keys: 'Shift+Any arrow', action: 'Movement step 1 mm' },
	{ keys: 'Esc', action: 'Deselect all' },
	{ keys: '1-9', action: 'Set filament for object/part' },
	{ keys: 'Ctrl+0', action: 'Camera view - Default' },
	{ keys: 'Ctrl+1', action: 'Camera view - Top' },
	{ keys: 'Ctrl+2', action: 'Camera view - Bottom' },
	{ keys: 'Ctrl+3', action: 'Camera view - Front' },
	{ keys: 'Ctrl+4', action: 'Camera view - Behind' },
	{ keys: 'Ctrl+5', action: 'Camera angle - Left' },
	{ keys: 'Ctrl+6', action: 'Camera angle - Right' },
	{ keys: 'Ctrl+A', action: 'Select all objects' },
	{ keys: 'Ctrl+D', action: 'Delete all' },
	{ keys: 'Ctrl+Z', action: 'Undo' },
	{ keys: 'Ctrl+Y', action: 'Redo' },
	{ keys: 'M', action: 'Gizmo move' },
	{ keys: 'R', action: 'Gizmo rotate' },
	{ keys: 'S', action: 'Gizmo scale' },
	{ keys: 'F', action: 'Gizmo place face on bed' },
	{ keys: 'C', action: 'Gizmo cut' },
	{ keys: 'B', action: 'Gizmo mesh boolean' },
	{ keys: 'H', action: 'Gizmo fuzzy skin' },
	{ keys: 'L', action: 'Gizmo SLA support points' },
	{ keys: 'P', action: 'Gizmo seam' },
	{ keys: 'T', action: 'Gizmo text emboss' },
	{ keys: 'U', action: 'Gizmo measure' },
	{ keys: 'Y', action: 'Gizmo assemble' },
	{ keys: 'E', action: 'Gizmo brim ears' },
	{ keys: 'I', action: 'Zoom in' },
	{ keys: 'O', action: 'Zoom out' },
	{ keys: 'V', action: 'Toggle printable' },
	{ keys: 'Tab', action: 'Switch Prepare/Preview' }
];

/**
 * Raccourcis de la liste d'objets — groupe « Objects » de l'annexe B §B.6
 * (Orca main aefb6b3d) + entrées 2.4.1 (« 1-9 », Espace, clic) toujours
 * câblées web.
 */
export const OBJECTS_LIST_SHORTCUTS: Shortcut[] = [
	{ keys: '1-9', action: 'Set extruder number' },
	{ keys: 'Del', action: 'Delete objects/parts/modifiers' },
	{ keys: 'Esc', action: 'Deselect all' },
	{ keys: 'Ctrl+C', action: 'Copy' },
	{ keys: 'Ctrl+V', action: 'Paste' },
	{ keys: 'Ctrl+X', action: 'Cut' },
	{ keys: 'Ctrl+A', action: 'Select all objects' },
	{ keys: 'Ctrl+K', action: 'Clone selected' },
	{ keys: 'Ctrl+Z', action: 'Undo' },
	{ keys: 'Ctrl+Y', action: 'Redo' },
	{ keys: 'Space', action: 'Rename selected' },
	{ keys: 'Mouse click', action: 'Rename on click' },
	{ keys: '+', action: 'Add instance' },
	{ keys: '-', action: 'Remove instance' },
	{ keys: 'V', action: 'Toggle printable for object/part' },
	{ keys: 'D', action: 'Auto Drop' },
	{ keys: 'Ctrl+Shift+R', action: 'Reload from disk' }
];

/**
 * Raccourcis du groupe Gizmos (Annexe B §B.6, Orca main aefb6b3d — « Gizmo »
 * 2.4.1 renommé « Gizmos », 15 touches) + interactions souris des gizmos de
 * peinture (toujours présentes, hors dialogue de raccourcis).
 */
export const GIZMO_SHORTCUTS: Shortcut[] = [
	{ keys: 'M', action: 'Gizmo move' },
	{ keys: 'R', action: 'Gizmo rotate' },
	{ keys: 'S', action: 'Gizmo scale' },
	{ keys: 'F', action: 'Gizmo place face on bed' },
	{ keys: 'C', action: 'Gizmo cut' },
	{ keys: 'B', action: 'Gizmo mesh boolean' },
	{ keys: 'L', action: 'Gizmo FDM paint-on supports' },
	{ keys: 'P', action: 'Gizmo FDM paint-on seam' },
	{ keys: 'H', action: 'Gizmo FDM paint-on fuzzy skin' },
	{ keys: 'N', action: 'Gizmo multi-material painting' },
	{ keys: 'T', action: 'Gizmo text emboss/engrave' },
	{ keys: 'U', action: 'Gizmo measure' },
	{ keys: 'Y', action: 'Gizmo assemble' },
	{ keys: 'E', action: 'Gizmo brim ears' },
	{ keys: 'D', action: 'Gizmo texture displacement painting' },
	// Interactions souris des gizmos (2.4.1, hors KBShortcutsDialog main) :
	{ keys: 'Esc', action: 'Deselect all' },
	{ keys: 'Shift+', action: 'Move: snap by 1 mm' },
	{ keys: 'Ctrl+Mouse wheel', action: 'Painting: adjust pen radius' },
	{ keys: 'Alt+Mouse wheel', action: 'Painting: adjust section position' }
];
