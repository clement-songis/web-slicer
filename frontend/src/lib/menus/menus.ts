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
	/**
	 * Description courte affichée sous le libellé (aide de la barre d'état
	 * d'Orca, `append_menu_item(…, help)`). Exclue du nom accessible (le libellé
	 * reste la clé de parité ciblée par les tests) et exposée en `aria-description`.
	 */
	hint?: string;
	/** Libellé de données (nom de projet) : affiché tel quel, jamais traduit. */
	raw?: boolean;
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
		// Orca : Ctrl+N, réservé par le navigateur → Alt+N (audit 41 S1, exclusions.md).
		{ label: 'New Project', shortcut: 'Alt+N', action: 'project.new' },
		{ label: 'Open Project…', shortcut: 'Ctrl+O', action: 'project.open' },
		// Audit 41 P2 : sous-menu « Recent files » d'Orca (MainFrame.cpp:2703,
		// max_recent_count) ; rempli à l'ouverture du menu (withRecentProjects).
		{ label: 'Recent files', action: 'file.recent', items: [] },
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
				{ label: 'Export G-code…', action: 'export.gcode' },
				{ label: 'Export plate sliced file…', shortcut: 'Ctrl+G', action: 'export.plate' },
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
		// Orca MainFrame « Show 3D Navigator » ; menu du canevas « Axes » (show_axes).
		{ label: 'Show 3D Navigator', action: 'view.navigator' },
		{ label: 'Show Axes', action: 'view.axes' },
		// Orca MainFrame : « Show &Labels\tCtrl+E ».
		{ label: 'Show Labels', shortcut: 'Ctrl+E', action: 'view.labels' },
		{ label: 'Show Overhang', action: 'view.overhang' },
		// Orca menu du canevas « Realistic View » (opengl_realistic_mode, phase 38).
		{ label: 'Realistic View', action: 'view.realistic' },
		{ label: 'Show Selected Outline (beta)', action: 'view.selectedOutline' }
	]
};

// Menu Calibration d'OrcaSlicer (MainFrame::init_menubar, calib_menu) : les
// calibrations exprimables sans `calib_params` moteur (cf. lib/calibration).
const calibrationMenu: Menu = {
	title: 'Calibration',
	entries: [
		// Libellés = clés de parité (inchangés, ciblés par les specs gelés) ; la
		// description (`hint`) dit ce que règle chaque test (phase 25). Groupes :
		// température/débit max, pressure advance, ratio de débit, mouvements.
		{
			label: 'Temperature',
			action: 'calib.temperature',
			hint: 'Temperature tower: best nozzle temperature'
		},
		{
			label: 'Max flowrate',
			action: 'calib.maxVolumetricSpeed',
			hint: 'Maximum volumetric speed of the filament (mm³/s)'
		},
		'separator',
		{
			label: 'Pressure advance',
			action: 'calib.pressureAdvance',
			hint: 'Tower: K value read from the best height'
		},
		{
			label: 'Pressure advance pattern',
			action: 'calib.paPattern',
			hint: 'Printed pattern: K value read from the sharpest corner'
		},
		// Historique des valeurs K (Orca : historique de calibration PA).
		{
			label: 'Pressure advance history',
			action: 'calib.paHistory',
			hint: 'K values saved per filament'
		},
		'separator',
		{
			label: 'Flow ratio',
			action: 'calib.flowRate',
			hint: 'Flow ratio wizard (YOLO or two passes)'
		},
		// Passes directes historiques (spec gelé editor-calibration-more) : ce
		// sont les deux passes de l'assistant « Flow ratio », lancées directement.
		{
			label: 'Flow rate (pass 1)',
			action: 'calib.flowPass1',
			hint: 'Flow ratio, coarse pass (-20 % to +20 %)'
		},
		{
			label: 'Flow rate (pass 2)',
			action: 'calib.flowPass2',
			hint: 'Flow ratio, fine pass after pass 1'
		},
		'separator',
		// Calibrations « mode moteur » (calib_params transmis au moteur).
		// « Retraction test » : libellé conservé tant que le spec gelé
		// editor-calibration-engine le cible (Orca : « Retraction »).
		{
			label: 'Retraction test',
			action: 'calib.retraction',
			hint: 'Retraction tower: shortest length without stringing'
		},
		{
			label: 'Cornering',
			action: 'calib.cornering',
			hint: 'Jerk / junction deviation in corners'
		},
		// Sous-menu « Input Shaping » d'Orca aplati : les specs gelés ciblent
		// directement l'entrée « Input Shaping Frequency ».
		{
			label: 'Input Shaping Frequency',
			action: 'calib.inputShapingFreq',
			hint: 'Resonance frequency per axis (Hz)'
		},
		{
			label: 'Input Shaping Damping/zeta factor',
			action: 'calib.inputShapingDamp',
			hint: 'Damping ratio of the input shaper'
		},
		{
			label: 'VFA',
			action: 'calib.vfa',
			hint: 'Vertical fine artifacts by speed'
		},
		'separator',
		{
			label: 'Calibration Guide',
			action: 'calib.guide',
			hint: 'OrcaSlicer documentation (new tab)'
		}
	]
};

const helpMenu: Menu = {
	title: 'Help',
	entries: [
		{ label: 'Keyboard Shortcuts', shortcut: '?', action: 'help.shortcuts' },
		{ label: 'Setup Wizard', action: 'help.setupWizard' },
		{ label: 'Show Tip of the Day', action: 'help.tipOfTheDay' },
		'separator',
		{ label: 'About', action: 'help.about' }
	]
};

/** Barre de menus principale (Annexe B §B.2, adaptée au navigateur). */
export const MAIN_MENUS: Menu[] = [fileMenu, editMenu, viewMenu, calibrationMenu, helpMenu];

/** Préfixe des actions « ouvrir le projet récent <id> ». */
export const RECENT_ACTION = 'project.openRecent:';

/**
 * Audit 42 V-a : liste « Recent files » d'Orca (add_to_recent_projects) — le
 * projet ouvert en tête, puis les autres dans l'ordre donné, sans doublon.
 */
export function recentFirst<T extends { id: string }>(projects: readonly T[], current: T): T[] {
	return [current, ...projects.filter((p) => p.id !== current.id)];
}

/**
 * Menus avec le sous-menu « Recent files » rempli : projets les plus récemment
 * modifiés (hors projet courant), bornés à `max` (Orca max_recent_count) ;
 * liste vide → entrée « No recent projects » sans effet.
 */
export function withRecentProjects(
	menus: readonly Menu[],
	recent: readonly { id: string; name: string }[],
	max: number
): Menu[] {
	const items: MenuItem[] = recent
		.slice(0, Math.max(0, max))
		.map((p) => ({ label: p.name, action: RECENT_ACTION + p.id, raw: true }));
	if (items.length === 0) items.push({ label: 'No recent projects', action: 'file.recent.none' });
	return menus.map((menu) =>
		menu.entries.some((e) => e !== 'separator' && e.action === 'file.recent')
			? {
					...menu,
					entries: menu.entries.map((e) =>
						e !== 'separator' && e.action === 'file.recent' ? { ...e, items } : e
					)
				}
			: menu
	);
}

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
