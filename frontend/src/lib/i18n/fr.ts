// Traduction française **additive** (T080, FR-072). Les clés sont les libellés
// de parité anglais d'OrcaSlicer (source de vérité) ; cette table les traduit.
// Toute clé absente retombe sur l'anglais (voir `translate`) — la parité prime,
// la traduction est un supplément.

/** Dictionnaire anglais → français (libellés d'interface). */
export const fr: Record<string, string> = {
	// Thème clair/sombre (T093)
	Theme: 'Thème',
	Light: 'Clair',
	Dark: 'Sombre',
	System: 'Système',

	// Titres de menus (Annexe B §B.2)
	File: 'Fichier',
	Edit: 'Édition',
	View: 'Vue',
	Help: 'Aide',

	// Menu Fichier
	'New Project': 'Nouveau projet',
	'Open Project…': 'Ouvrir un projet…',
	'Save Project': 'Enregistrer le projet',
	'Save Project as…': 'Enregistrer le projet sous…',
	Import: 'Importer',
	'Import 3MF/STL/STEP/SVG/OBJ/AMF…': 'Importer 3MF/STL/STEP/SVG/OBJ/AMF…',
	'Import Zip Archive…': 'Importer une archive Zip…',
	'Import Configs…': 'Importer des configurations…',
	Export: 'Exporter',
	'Export G-code…': 'Exporter le G-code…',
	'Export plate sliced file…': 'Exporter le fichier tranché du plateau…',
	'Export all plate sliced file…': 'Exporter les fichiers tranchés de tous les plateaux…',
	'Export all objects as one STL…': 'Exporter tous les objets en un seul STL…',
	'Export all objects as STLs…': 'Exporter tous les objets en STL séparés…',
	'Export all objects as one DRC…': 'Exporter tous les objets en un seul DRC…',
	'Export all objects as DRCs…': 'Exporter tous les objets en DRC séparés…',
	'Export Generic 3MF…': 'Exporter en 3MF générique…',
	'Export toolpaths as OBJ…': 'Exporter les trajectoires en OBJ…',
	'Export Preset Bundle…': 'Exporter le lot de presets…',
	'Open G-code…': 'Ouvrir un G-code…',
	'Reload from Disk…': 'Recharger depuis le disque…',
	'Preset Bundle': 'Lot de presets',
	Preferences: 'Préférences',

	// Menu Édition
	Undo: 'Annuler',
	Redo: 'Rétablir',
	Cut: 'Couper',
	Copy: 'Copier',
	Paste: 'Coller',
	'Delete selected': 'Supprimer la sélection',
	'Delete all': 'Tout supprimer',
	'Clone selected': 'Cloner la sélection',
	'Duplicate Current Plate': 'Dupliquer le plateau courant',
	'Select all': 'Tout sélectionner',
	'Deselect all': 'Tout désélectionner',

	// Menu Vue
	'Default View': 'Vue par défaut',
	Top: 'Dessus',
	Bottom: 'Dessous',
	Front: 'Face',
	Rear: 'Arrière',
	Left: 'Gauche',
	Right: 'Droite',
	'Use Perspective View': 'Vue en perspective',
	'Use Orthogonal View': 'Vue orthogonale',
	'Auto Perspective': 'Perspective automatique',
	'Show G-code Window': 'Afficher la fenêtre G-code',
	'Show Gridlines': 'Afficher la grille',
	'Show Labels': 'Afficher les étiquettes',
	'Show Overhang': 'Afficher les surplombs',
	'Show Selected Outline (beta)': 'Afficher le contour de la sélection (bêta)',

	// Menu Aide
	'Keyboard Shortcuts': 'Raccourcis clavier',

	// Groupes de raccourcis (Annexe B §B.6)
	'Global shortcuts': 'Raccourcis globaux',
	Plater: 'Plateau',
	Gizmo: 'Gizmo',
	'Objects List': "Liste d'objets",
	Preview: 'Prévisualisation',

	// Chrome applicatif (hors parité OrcaSlicer)
	Close: 'Fermer'
};
