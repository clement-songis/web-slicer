// Menus principaux de la barre de menus (T079, Annexe B §B.2) : Fichier, Édition,
// Vue, Aide (+ sous-menus Import/Export). Les libellés anglais sont les clés de
// parité (issus d'OrcaSlicer, `MainFrame::init_menubar`) ; l'affichage localisé
// est une couche i18n (T080). La traçabilité des libellés vers ce fichier est
// dans `specs/001-orcaslicer-web-parity/traceability-map.json` (vérifiée par
// `audit/check_traceability.py`) ; les items sans objet en navigateur sont
// consignés dans `exclusions.md`.
//
// Aucune logique métier ici : les `action` sont des identifiants stables que la
// coquille applicative câble vers les commandes (import, export, vue, undo…).

/** Élément de menu (feuille) : libellé, raccourci affiché, identifiant d'action. */
export interface MenuItem {
	label: string;
	/** Raccourci affiché (forme OrcaSlicer). */
	shortcut?: string;
	/** Identifiant d'action stable (câblé par la coquille). */
	action: string;
	/** Sous-menu éventuel (Import/Export). */
	items?: MenuItem[];
}

/** Séparateur visuel entre groupes d'items. */
export type MenuEntry = MenuItem | 'separator';

/** Menu de la barre principale. */
export interface Menu {
	title: string;
	entries: MenuEntry[];
}

const fileMenu: Menu = {
	title: 'File',
	entries: [
		{ label: 'New Project', shortcut: 'Ctrl+N', action: 'project.new' },
		{ label: 'Open Project…', shortcut: 'Ctrl+O', action: 'project.open' },
		{ label: 'Save Project', shortcut: 'Ctrl+S', action: 'project.save' },
		{ label: 'Save Project as…', shortcut: 'Ctrl+Shift+S', action: 'project.saveAs' },
		'separator',
		{
			label: 'Import',
			action: 'file.import',
			items: [
				{
					label: 'Import 3MF/STL/STEP/SVG/OBJ/AMF…',
					shortcut: 'Ctrl+I',
					action: 'import.geometry'
				},
				{ label: 'Import Zip Archive…', action: 'import.zip' },
				{ label: 'Import Configs…', action: 'import.configs' }
			]
		},
		{
			label: 'Export',
			action: 'file.export',
			items: [
				{ label: 'Export G-code…', shortcut: 'Ctrl+G', action: 'export.gcode' },
				{ label: 'Export plate sliced file…', action: 'export.plate' },
				{ label: 'Export all plate sliced file…', action: 'export.allPlates' },
				{ label: 'Export all objects as one STL…', action: 'export.stlOne' },
				{ label: 'Export all objects as STLs…', action: 'export.stlMany' },
				{ label: 'Export all objects as one DRC…', action: 'export.drcOne' },
				{ label: 'Export all objects as DRCs…', action: 'export.drcMany' },
				{ label: 'Export Generic 3MF…', action: 'export.generic3mf' },
				{ label: 'Export toolpaths as OBJ…', action: 'export.toolpathsObj' },
				{ label: 'Export Preset Bundle…', action: 'export.presetBundle' }
			]
		},
		'separator',
		{ label: 'Open G-code…', action: 'gcode.open' },
		{ label: 'Reload from Disk…', shortcut: 'F5', action: 'project.reload' },
		{ label: 'Preset Bundle', action: 'preset.bundle' },
		'separator',
		{ label: 'Preferences', shortcut: 'Ctrl+P', action: 'app.preferences' }
	]
};

const editMenu: Menu = {
	title: 'Edit',
	entries: [
		{ label: 'Undo', shortcut: 'Ctrl+Z', action: 'edit.undo' },
		{ label: 'Redo', shortcut: 'Ctrl+Y', action: 'edit.redo' },
		'separator',
		{ label: 'Cut', shortcut: 'Ctrl+X', action: 'edit.cut' },
		{ label: 'Copy', shortcut: 'Ctrl+C', action: 'edit.copy' },
		{ label: 'Paste', shortcut: 'Ctrl+V', action: 'edit.paste' },
		'separator',
		{ label: 'Delete selected', shortcut: 'Del', action: 'edit.deleteSelected' },
		{ label: 'Delete all', shortcut: 'Ctrl+D', action: 'edit.deleteAll' },
		{ label: 'Clone selected', shortcut: 'Ctrl+K', action: 'edit.clone' },
		{ label: 'Duplicate Current Plate', action: 'edit.duplicatePlate' },
		'separator',
		{ label: 'Select all', shortcut: 'Ctrl+A', action: 'edit.selectAll' },
		{ label: 'Deselect all', shortcut: 'Esc', action: 'edit.deselectAll' }
	]
};

const viewMenu: Menu = {
	title: 'View',
	entries: [
		{ label: 'Default View', shortcut: 'Ctrl+0', action: 'view.default' },
		{ label: 'Top', shortcut: 'Ctrl+1', action: 'view.top' },
		{ label: 'Bottom', shortcut: 'Ctrl+2', action: 'view.bottom' },
		{ label: 'Front', shortcut: 'Ctrl+3', action: 'view.front' },
		{ label: 'Rear', shortcut: 'Ctrl+4', action: 'view.rear' },
		{ label: 'Left', shortcut: 'Ctrl+5', action: 'view.left' },
		{ label: 'Right', shortcut: 'Ctrl+6', action: 'view.right' },
		'separator',
		{ label: 'Use Perspective View', action: 'view.perspective' },
		{ label: 'Use Orthogonal View', action: 'view.orthogonal' },
		{ label: 'Auto Perspective', action: 'view.autoPerspective' },
		'separator',
		{ label: 'Show G-code Window', shortcut: 'C', action: 'view.gcodeWindow' },
		{ label: 'Show Gridlines', action: 'view.gridlines' },
		{ label: 'Show Labels', action: 'view.labels' },
		{ label: 'Show Overhang', action: 'view.overhang' },
		{ label: 'Show Selected Outline (beta)', action: 'view.selectedOutline' }
	]
};

const helpMenu: Menu = {
	title: 'Help',
	entries: [{ label: 'Keyboard Shortcuts', shortcut: '?', action: 'help.shortcuts' }]
};

/** Barre de menus principale (Annexe B §B.2, adaptée au navigateur). */
export const MAIN_MENUS: Menu[] = [fileMenu, editMenu, viewMenu, helpMenu];

/** Aplati les libellés d'un menu (items + sous-items) — utile aux tests. */
export function menuLabels(menu: Menu): string[] {
	const out: string[] = [];
	for (const entry of menu.entries) {
		if (entry === 'separator') continue;
		out.push(entry.label);
		for (const sub of entry.items ?? []) out.push(sub.label);
	}
	return out;
}

/** Tous les libellés d'action de la barre de menus (feuilles et sous-menus). */
export function allMenuLabels(): string[] {
	return MAIN_MENUS.flatMap(menuLabels);
}
