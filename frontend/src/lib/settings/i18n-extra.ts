// Compléments FR absents du dictionnaire généré (scripts/gen-settings-i18n.mjs) :
// libellés dont la chaîne anglaise exacte n'est pas une msgid d'OrcaSlicer_fr.po
// (types de plaque de la table des températures, valeurs d'énumération). Les
// traductions reprennent les msgstr d'Orca les plus proches (« Cool Plate »,
// « Textured PEI Plate », « Engineering Plate »…). Repli anglais sinon.
export const SETTINGS_FR_EXTRA: Record<string, string> = {
	'Cool Plate / PLA Plate': 'Cool Plate / Plaque PLA',
	// msgid présente dans le .po (« Extrudeur ») mais écartée par le générateur
	// (chaîne d'interface) : libellé de la clé `extruder` par objet / par plage.
	Extruder: 'Extrudeur',
	'Supertack Plate': 'Plaque SuperTack',
	'Engineering Plate': 'Plaque Engineering',
	'Smooth High Temp Plate': 'Plaque lisse haute température',
	'Textured PEI Plate': 'Plaque PEI texturée',
	'Textured Cool Plate': 'Plaque Cool plate texturée',
	'Smooth Cool Plate': 'Plaque Cool plate lisse',
	'Cool Plate (SuperTack)': 'Cool Plate (SuperTack)',
	// Unités (sidetext) sans msgid exact dans le .po d'Orca.
	// (msgstr d'OrcaSlicer_fr.po pour ces unités, absentes du dictionnaire généré)
	'mm or %': 'mm ou %',
	'mm/s or %': 'mm/s ou %',
	'mm/s² or %': 'mm/s² ou %',
	layers: 'couches',
	layer: 'couche',
	'money/kg': 'argent/kg',
	'money/h': '€/heure',
	// Catégories de réglages (en-têtes de la recherche « + Réglage ») : msgstr
	// d'OrcaSlicer_fr.po, absentes du dictionnaire généré (chaînes d'interface).
	Extruders: 'Extrudeurs',
	'Layers and Perimeters': 'Couches et Périmètres',
	'Machine limits': 'Limites de la machine',
	Other: 'Autre',
	// Phase 27 — msgid sans msgstr (ou msgstr fautive) dans OrcaSlicer_fr.po 2.4.1.
	// Phase 28 — msgstr d'Orca citant les valeurs anglaises « Default » / « Disable »
	// alors que la liste affiche leurs traductions (« Défaut », « Désactiver »).
	'Choose the input shaper algorithm.\nDefault uses the firmware default settings.\nDisable turns off input shaping in the firmware.':
		'Choisissez l’algorithme de mise en forme du signal.\n« Défaut » utilise les réglages par défaut du firmware.\n« Désactiver » désactive la mise en forme du signal dans le firmware.',
	'Printer notes': 'Notes de l’imprimante',
	'Use 3MF instead of G-code': 'Utiliser un 3MF au lieu du G-code',
	'Enable this if the printer accepts a 3MF file as the print job. When enabled, Orca Slicer sends the sliced file as a .gcode.3mf, instead of a plain .gcode file.':
		'À activer si l’imprimante accepte un fichier 3MF comme tâche d’impression : le fichier tranché est alors envoyé en .gcode.3mf au lieu d’un simple .gcode.',
	'Tightens the gyroid wave along the Z (vertical) axis at low infill density to shorten the effective vertical column length and improve Z-axis compression buckling resistance. Filament use is preserved. No effect at ~30% sparse infill density and above. Only applies when Sparse infill pattern is set to Gyroid.':
		'Resserre l’onde du gyroïde selon l’axe Z (vertical) aux faibles densités de remplissage pour raccourcir la longueur effective des colonnes verticales et améliorer la résistance au flambage en compression selon Z. La quantité de filament est inchangée. Sans effet à partir d’environ 30 % de densité de remplissage. Ne s’applique qu’au motif de remplissage Gyroïde.'
};
