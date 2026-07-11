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

/** Libellés du menu contextuel du plateau (Annexe B §B.3, dédupliqués). */
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
	'Height range Modifier',
	'Set as an individual object',
	'Fill bed with copies',
	'Printable',
	'Rename',
	'Fix model',
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
	'Mesh boolean',
	'Along X axis',
	'Along Y axis',
	'Along Z axis',
	'Add Models',
	'Show Labels',
	'To objects',
	'To parts',
	'Split',
	'Auto orientation',
	'Edit',
	'Select All',
	'Select All Plates',
	'Delete All',
	'Arrange',
	'Reload All',
	'Auto Rotate',
	'Delete Plate',
	'Add instance',
	'Remove instance',
	'Set number of instances…',
	'Fill bed with instances…',
	'Clone',
	'Simplify Model',
	'Subdivision mesh(Lost color)',
	'Center',
	'Drop'
].map((label) => ({ label }));

/** Entrée du menu contextuel objet (T112) : libellé de parité + identifiant d'action. */
export interface ObjectMenuItem {
	label: string;
	action: string;
}

/** Entrée du menu contextuel objet, avec séparateurs. */
export type ObjectMenuEntry = ObjectMenuItem | 'separator';

/**
 * Actions objet du menu contextuel (sous-ensemble objet de `context_menu`, T112).
 * Libellés = clés de parité (tracés dans `traceability-map.json#context_menu`).
 */
export const OBJECT_CONTEXT_ITEMS: ObjectMenuEntry[] = [
	{ label: 'HideShow', action: 'object.hideShow' },
	{ label: 'Rename', action: 'object.rename' },
	{ label: 'Edit', action: 'object.edit' },
	'separator',
	{ label: 'Clone', action: 'object.clone' },
	{ label: 'Add instance', action: 'object.addInstance' },
	{ label: 'Remove instance', action: 'object.removeInstance' },
	'separator',
	{ label: 'Auto orientation', action: 'object.orient' },
	{ label: 'Center', action: 'object.center' },
	{ label: 'Drop', action: 'object.drop' },
	'separator',
	{ label: 'Fix model', action: 'object.fix' },
	{ label: 'Simplify Model', action: 'object.simplify' },
	{ label: 'To objects', action: 'object.toObjects' },
	{ label: 'To parts', action: 'object.toParts' },
	{ label: 'Split', action: 'object.split' },
	'separator',
	{ label: 'Delete', action: 'object.delete' }
];

/** Raccourci clavier : combinaison (verbatim Annexe B) → action. */
export interface Shortcut {
	keys: string;
	action: string;
}

/** Raccourcis du plateau (Annexe B §B.6 « Plater »). */
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
	{ keys: 'Arrow Up', action: 'Move +Y 10 mm' },
	{ keys: 'Arrow Down', action: 'Move -Y 10 mm' },
	{ keys: 'Arrow Left', action: 'Move -X 10 mm' },
	{ keys: 'Arrow Right', action: 'Move +X 10 mm' },
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

/** Raccourcis de la liste d'objets (Annexe B §B.6 « Objects List »). */
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
	{ keys: 'Mouse click', action: 'Rename on click' }
];

/** Raccourcis du groupe Gizmo (Annexe B §B.6 « Gizmo »). */
export const GIZMO_SHORTCUTS: Shortcut[] = [
	{ keys: 'Esc', action: 'Deselect all' },
	{ keys: 'Shift+', action: 'Move: snap by 1 mm' },
	{ keys: 'Ctrl+Mouse wheel', action: 'Painting: adjust pen radius' },
	{ keys: 'Alt+Mouse wheel', action: 'Painting: adjust section position' }
];
