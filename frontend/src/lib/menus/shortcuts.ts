// Contenu de l'aide raccourcis (T079, Annexe B §B.6) : les 92 raccourcis
// d'OrcaSlicer regroupés (Global, Plater, Gizmo, Objects List, Preview),
// affichés verbatim par `ShortcutsDialog.svelte`. Source de vérité :
// `audit/ui_inventory.json` (keyboard_shortcuts) ; les touches en conflit
// navigateur (Ctrl+Tab, Ctrl+M/3Dconnexion) sont consignées dans exclusions.md
// et remappées in-app. Aucune logique métier : données d'affichage.

/** Un raccourci : combinaison de touches (forme OrcaSlicer) + action décrite. */
export interface Shortcut {
	keys: string;
	action: string;
}

/** Groupe de raccourcis (onglet/section de l'aide). */
export interface ShortcutGroup {
	group: string;
	shortcuts: Shortcut[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
	{
		group: 'Global shortcuts',
		shortcuts: [
			{ keys: 'Ctrl+N', action: 'New Project' },
			{ keys: 'Ctrl+O', action: 'Open Project' },
			{ keys: 'Ctrl+S', action: 'Save Project' },
			{ keys: 'Ctrl+Shift+S', action: 'Save Project as' },
			{ keys: 'Ctrl+I', action: 'Import geometry data from STL/STEP/3MF/OBJ/AMF files' },
			{ keys: 'Ctrl+G', action: 'Export plate sliced file' },
			{ keys: 'Ctrl+R', action: 'Slice plate' },
			{ keys: 'Ctrl+Shift+G', action: 'Print plate' },
			{ keys: 'Ctrl+X', action: 'Cut' },
			{ keys: 'Ctrl+C', action: 'Copy to clipboard' },
			{ keys: 'Ctrl+V', action: 'Paste from clipboard' },
			{ keys: 'Ctrl+P', action: 'Preferences' },
			{ keys: 'Ctrl+Shift+M', action: 'Show/Hide 3Dconnexion devices settings dialog' },
			{ keys: 'Ctrl+M', action: 'Show/Hide 3Dconnexion devices settings dialog' },
			{ keys: 'Ctrl+Tab', action: 'Switch table page' },
			{ keys: 'fn+⌫', action: 'Delete selected' },
			{ keys: 'Del', action: 'Delete selected' },
			{ keys: '?', action: 'Show keyboard shortcuts list' }
		]
	},
	{
		group: 'Plater',
		shortcuts: [
			{ keys: 'Mouse wheel', action: 'Zoom View' },
			{ keys: 'A', action: 'Arrange all objects' },
			{ keys: 'Shift+A', action: 'Arrange objects on selected plates' },
			{
				keys: 'Q',
				action:
					'Auto orients selected objects or all objects. If there are selected objects, it just orients the selected ones. Otherwise, it will orient all objects in the current project.'
			},
			{ keys: 'Shift+Q', action: 'Auto orients all objects on the active plate.' },
			{ keys: 'Shift+Tab', action: 'Collapse/Expand the sidebar' },
			{ keys: 'Ctrl+Any arrow', action: 'Movement in camera space' },
			{ keys: 'Alt+Left mouse button', action: 'Select a part' },
			{ keys: 'Ctrl+Left mouse button', action: 'Select multiple objects' },
			{ keys: 'Shift+Left mouse button', action: 'Select objects by rectangle' },
			{ keys: 'Arrow Up', action: 'Move selection 10 mm in positive Y direction' },
			{ keys: 'Arrow Down', action: 'Move selection 10 mm in negative Y direction' },
			{ keys: 'Arrow Left', action: 'Move selection 10 mm in negative X direction' },
			{ keys: 'Arrow Right', action: 'Move selection 10 mm in positive X direction' },
			{ keys: 'Shift+Any arrow', action: 'Movement step set to 1 mm' },
			{ keys: 'Esc', action: 'Deselect all' },
			{ keys: '1-9', action: 'Keyboard 1-9: set filament for object/part' },
			{ keys: 'Ctrl+0', action: 'Camera view - Default' },
			{ keys: 'Ctrl+1', action: 'Camera view - Top' },
			{ keys: 'Ctrl+2', action: 'Camera view - Bottom' },
			{ keys: 'Ctrl+3', action: 'Camera view - Front' },
			{ keys: 'Ctrl+4', action: 'Camera view - Behind' },
			{ keys: 'Ctrl+5', action: 'Camera Angle - Left side' },
			{ keys: 'Ctrl+6', action: 'Camera Angle - Right side' },
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
			{ keys: 'H', action: 'Gizmo FDM paint-on fuzzy skin' },
			{ keys: 'L', action: 'Gizmo SLA support points' },
			{ keys: 'P', action: 'Gizmo FDM paint-on seam' },
			{ keys: 'T', action: 'Gizmo text emboss/engrave' },
			{ keys: 'U', action: 'Gizmo measure' },
			{ keys: 'Y', action: 'Gizmo assemble' },
			{ keys: 'E', action: 'Gizmo brim ears' },
			{ keys: 'I', action: 'Zoom in' },
			{ keys: 'O', action: 'Zoom out' },
			{ keys: 'V', action: 'Toggle printable for object/part' },
			{ keys: 'Tab', action: 'Switch between Prepare/Preview' }
		]
	},
	{
		group: 'Gizmo',
		shortcuts: [
			{ keys: 'Esc', action: 'Deselect all' },
			{ keys: 'Shift+', action: 'Move: press to snap by 1mm' },
			{ keys: 'Ctrl+Mouse wheel', action: 'Support/Color Painting: adjust pen radius' },
			{ keys: 'Alt+Mouse wheel', action: 'Support/Color Painting: adjust section position' }
		]
	},
	{
		group: 'Objects List',
		shortcuts: [
			{ keys: '1-9', action: 'Set extruder number for the objects and parts' },
			{ keys: 'Del', action: 'Delete objects, parts, modifiers' },
			{ keys: 'Esc', action: 'Deselect all' },
			{ keys: 'Ctrl+C', action: 'Copy to clipboard' },
			{ keys: 'Ctrl+V', action: 'Paste from clipboard' },
			{ keys: 'Ctrl+X', action: 'Cut' },
			{ keys: 'Ctrl+A', action: 'Select all objects' },
			{ keys: 'Ctrl+K', action: 'Clone selected' },
			{ keys: 'Ctrl+Z', action: 'Undo' },
			{ keys: 'Ctrl+Y', action: 'Redo' },
			{ keys: 'Space', action: 'Select the object/part and press space to change the name' },
			{ keys: 'Mouse click', action: 'Select the object/part and mouse click to change the name' }
		]
	},
	{
		group: 'Preview',
		shortcuts: [
			{ keys: 'Arrow Up', action: 'Vertical slider - Move active thumb Up' },
			{ keys: 'Arrow Down', action: 'Vertical slider - Move active thumb Down' },
			{ keys: 'Arrow Left', action: 'Horizontal slider - Move active thumb Left' },
			{ keys: 'Arrow Right', action: 'Horizontal slider - Move active thumb Right' },
			{ keys: 'L', action: 'On/Off one layer mode of the vertical slider' },
			{ keys: 'C', action: 'On/Off G-code window' },
			{ keys: 'Tab', action: 'Switch between Prepare/Preview' },
			{ keys: 'Shift+Any arrow', action: 'Move slider 5x faster' },
			{ keys: 'Shift+Mouse wheel', action: 'Move slider 5x faster' },
			{ keys: 'Ctrl+Any arrow', action: 'Move slider 5x faster' },
			{ keys: 'Ctrl+Mouse wheel', action: 'Move slider 5x faster' },
			{ keys: 'Home', action: 'Horizontal slider - Move to start position' },
			{ keys: 'End', action: 'Horizontal slider - Move to last position' }
		]
	}
];
/** Nombre total de raccourcis répertoriés (parité : 92, Annexe B §B.6). */
export function totalShortcuts(): number {
	return SHORTCUT_GROUPS.reduce((n, g) => n + g.shortcuts.length, 0);
}
