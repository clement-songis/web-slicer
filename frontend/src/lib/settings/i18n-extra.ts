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
	Other: 'Autre'
};
