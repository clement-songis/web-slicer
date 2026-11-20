// Traductions françaises de l'aide « Raccourcis clavier » (phase 26) : la
// boîte affichait les 92 descriptions et noms de touches en anglais dans une
// interface française. Chaînes reprises telles quelles des msgstr
// d'OrcaSlicer_fr.po (OrcaSlicer 2.4.1, KBShortcutsDialog) — générées par
// .work/staging/design-system/phase26/gen-shortcuts-fr.mjs. Repli sur l'anglais.
import type { Locale } from '$lib/i18n';

export const SHORTCUTS_FR: Record<string, string> = {
	// Entrée multi-ligne du .po (msgid sur trois lignes).
	'Auto orients selected objects or all objects. If there are selected objects, it just orients the selected ones. Otherwise, it will orient all objects in the current project.':
		'Oriente automatiquement les objets sélectionnés ou tous les objets. S’il y a des objets sélectionnés, seuls ceux-ci sont orientés. Sinon, tous les objets du projet en cours sont orientés.',
	'Any arrow': 'Toutes les flèches',
	'Arrange all objects': 'Organiser tous les objets',
	'Arrange objects on selected plates': 'Organiser les objets sur les plaques sélectionnées',
	'Arrow Down': 'Flèche Bas',
	'Arrow Left': 'Flèche Gauche',
	'Arrow Right': 'Flèche Droite',
	'Arrow Up': 'Flèche Haut',
	'Auto orients all objects on the active plate.':
		'Oriente automatiquement tous les objets du plaque actuelle.',
	'Camera Angle - Left side': 'Angle de caméra - Côté gauche',
	'Camera Angle - Right side': 'Angle de caméra - Côté droit',
	'Camera view - Behind': 'Vue caméra - Derrière',
	'Camera view - Bottom': 'Vue caméra - Bas',
	'Camera view - Default': 'Vue caméra - Par défaut',
	'Camera view - Front': 'Vue de la caméra - Avant',
	'Camera view - Top': 'Vue caméra - Haut',
	'Clone selected': 'Cloner sélectionné',
	'Collapse/Expand the sidebar': 'Réduire/développer la barre latérale',
	'Copy to clipboard': 'Copier dans le presse-papier',
	Cut: 'Couper',
	Del: 'Eff',
	'Delete all': 'Tout supprimer',
	'Delete objects, parts, modifiers': 'Supprimer des objets, des pièces, des modificateurs',
	'Delete selected': 'Supprimer la sélection',
	'Deselect all': 'Désélectionner tout',
	End: 'Fin',
	Esc: 'Échap',
	'Export plate sliced file': 'Exporter le fichier découpé du plateau',
	'Gizmo FDM paint-on fuzzy skin': 'Outil FDM de surface irrégulière peinte',
	'Gizmo FDM paint-on seam': 'Gizmo Peinture de la couture FDM',
	'Gizmo SLA support points': 'Gizmo Point de support SLA',
	'Gizmo assemble': 'Gizmo assembler',
	'Gizmo brim ears': 'Gizmo bordure à oreilles',
	'Gizmo cut': 'Gizmo couper',
	'Gizmo measure': 'Gizmo mesurer',
	'Gizmo mesh boolean': 'Gizmo booléennes',
	'Gizmo move': 'Gizmo déplacer',
	'Gizmo place face on bed': 'Gizmo placer la face sur le plateau',
	'Gizmo rotate': 'Gizmo pivoter',
	'Gizmo scale': 'Gizmo redimensionner',
	'Gizmo text emboss/engrave': 'Gizmo embosser/graver du texte',
	'Global shortcuts': 'Raccourcis globaux',
	Home: 'Accueil',
	'Horizontal slider - Move active thumb Left':
		'Barre de défilement horizontale - Déplacer le curseur actif vers la Gauche',
	'Horizontal slider - Move active thumb Right':
		'Barre de défilement horizontale - Déplacer le curseur actif vers la Droite',
	'Horizontal slider - Move to last position':
		'Curseur horizontal - Déplacer vers la dernière position',
	'Horizontal slider - Move to start position':
		'Curseur horizontal - Déplacement vers la position de départ',
	'Import geometry data from STL/STEP/3MF/OBJ/AMF files':
		'Importez des données de géométrie à partir de fichiers STL/STEP/3MF/OBJ/AMF',
	'Keyboard 1-9: set filament for object/part':
		"clavier 1-9 : définir le filament pour l'objet/la pièce",
	'Left mouse button': 'Bouton gauche de la souris',
	'Mouse click': 'Clic de souris',
	'Mouse wheel': 'Molette de souris',
	'Move selection 10 mm in negative X direction':
		'Déplacer la sélection de 10 mm dans la direction négative X',
	'Move selection 10 mm in negative Y direction':
		'Déplacer la sélection de 10 mm dans la direction négative Y',
	'Move selection 10 mm in positive X direction':
		'Déplacer la sélection de 10 mm dans la direction positive X',
	'Move selection 10 mm in positive Y direction':
		'Déplacer la sélection de 10 mm dans la direction positive Y',
	'Move slider 5x faster': 'Déplacez le curseur 5 fois plus vite',
	'Move: press to snap by 1mm': 'Déplacer : appuyez pour aligner de 1 mm',
	'Movement in camera space': "Mouvement dans l'espace de la caméra",
	'Movement step set to 1 mm': 'Pas du mouvement réglé sur 1 mm',
	'New Project': 'Nouveau projet',
	'Objects List': 'Liste des objets',
	'On/Off G-code window': 'On/Off Fenêtre G-code',
	'On/Off one layer mode of the vertical slider':
		'On/Off mode couche unique de la barre de défilement verticale',
	'Open Project': 'Ouvrir un projet',
	'Paste from clipboard': 'Coller depuis le presse-papier',
	Plater: 'Plateau',
	Preferences: 'Préférences',
	Preview: 'Aperçu',
	'Print plate': 'Imprimer le plateau',
	Redo: 'Recommencer',
	'Save Project': 'Enregistrer le projet',
	'Save Project as': 'Enregistrer le projet sous',
	'Select a part': 'Sélectionner une pièce',
	'Select all objects': 'Sélectionner tous les objets',
	'Select multiple objects': 'Sélectionnez tous les objets sur la plaque actuelle',
	'Select objects by rectangle': 'Sélectionner les objets par rectangle',
	'Select the object/part and mouse click to change the name':
		"Sélectionnez l'objet/la pièce et cliquez avec la souris pour changer le nom",
	'Select the object/part and press space to change the name':
		"Sélectionnez l'objet/la pièce et appuyez sur espace pour changer le nom",
	'Set extruder number for the objects and parts':
		"Définir le numéro d'extrudeur pour les objets et les pièces",
	'Show keyboard shortcuts list': 'Afficher la liste des raccourcis clavier',
	'Show/Hide 3Dconnexion devices settings dialog':
		'Afficher/Masquer la boîte de dialogue des paramètres des périphériques 3Dconnexion',
	'Slice plate': 'Découper le plateau',
	Space: 'Espace',
	'Support/Color Painting: adjust pen radius':
		'Support/Peinture couleur : ajustez le rayon du stylet',
	'Support/Color Painting: adjust section position':
		'Support/Peinture couleur : ajuster la position de la section',
	'Switch between Prepare/Preview': 'Basculer entre Préparer/Aperçu',
	'Switch table page': 'Page du tableau de commutation',
	'Toggle printable for object/part': 'Basculer l’état imprimable de l’objet/de la pièce',
	Undo: 'Annuler',
	'Vertical slider - Move active thumb Down':
		'Barre de défilement verticale - Déplacer le curseur actif vers le Bas',
	'Vertical slider - Move active thumb Up':
		'Barre de défilement verticale - Déplacer le curseur actif vers le Haut',
	'Zoom View': 'Zoom de la vue',
	'Zoom in': 'Zoom avant',
	'Zoom out': 'Zoom arrière',
	// Phase 34 : groupe « Autres raccourcis » (hors liste KBShortcutsDialog d'Orca).
	'More shortcuts': 'Autres raccourcis',
	'Show/Hide object labels': 'Afficher/masquer les étiquettes d’objets',
	'Camera view - Current plate': 'Vue caméra - Plateau courant',
	'Page Up': 'Page préc.',
	'Page Down': 'Page suiv.',
	'Rotate selection 45° counterclockwise': 'Tourner la sélection de 45° (sens antihoraire)',
	'Rotate selection 45° clockwise': 'Tourner la sélection de 45° (sens horaire)',
	'Add an instance of the selected object': 'Ajouter une instance de l’objet sélectionné',
	'Remove an instance of the selected object': 'Retirer une instance de l’objet sélectionné',
	'Preview: add a color change at the current layer':
		'Aperçu : ajouter un changement de couleur à la couche courante',
	'Preview: delete the code at the current layer':
		'Aperçu : supprimer le code de la couche courante'
};

/** Description, groupe ou nom de touche traduit (repli : texte d'origine). */
export function shortcutText(text: string, locale: Locale): string {
	return locale === 'fr' ? (SHORTCUTS_FR[text] ?? text) : text;
}

/** Combinaison « Ctrl+Left mouse button » : chaque touche traduite séparément. */
export function shortcutKeysText(keys: string, locale: Locale): string {
	if (locale !== 'fr' || keys.length <= 1) return keys;
	return keys
		.split('+')
		.map((k) => shortcutText(k, locale))
		.join('+');
}
