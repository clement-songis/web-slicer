// Compléments FR absents du dictionnaire généré (scripts/gen-settings-i18n.mjs) :
// libellés dont la chaîne anglaise exacte n'est pas une msgid d'OrcaSlicer_fr.po
// (types de plaque de la table des températures, valeurs d'énumération). Les
// traductions reprennent les msgstr d'Orca les plus proches (« Cool Plate »,
// « Textured PEI Plate », « Engineering Plate »…). Repli anglais sinon.
export const SETTINGS_FR_EXTRA: Record<string, string> = {
	// Audit 48 DLG48-a : valeurs d'énumération de `timelapse_type` (dialogue
	// « Réglages du plateau ») — OrcaSlicer_fr.po : « Traditional » →
	// « Traditionnel », « Smooth » → « Lisse ».
	Traditional: 'Traditionnel',
	Smooth: 'Lisse',
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
	// Libellés des lignes composées d'Orca (UI_LINES, Tab.cpp) : msgstr d'OrcaSlicer_fr.po
	// (« Target » / « Minimal » n'y sont pas : traduction directe).
	Nozzle: 'Buse',
	'Cool Plate': 'Cool Plate/Plaque PLA',
	'Smooth PEI Plate / High Temp Plate': 'Plateau PEI lisse / Plateau haute température',
	'Recommended nozzle temperature': 'Température de buse recommandée',
	Target: 'Cible',
	Minimal: 'Minimale',
	'Min fan speed threshold': 'Seuil de vitesse mini du ventilateur',
	'Max fan speed threshold': 'Seuil de vitesse maximale du ventilateur',
	'Complete print': 'Après l’impression',
	'Fan speed-up time': 'Durée d’accélération du ventilateur',
	'Resonance Avoidance Speed': 'Vitesse d’évitement de résonance',
	Frequency: 'Fréquence',
	Damping: 'Amortissement',
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
		'Resserre l’onde du gyroïde selon l’axe Z (vertical) aux faibles densités de remplissage pour raccourcir la longueur effective des colonnes verticales et améliorer la résistance au flambage en compression selon Z. La quantité de filament est inchangée. Sans effet à partir d’environ 30 % de densité de remplissage. Ne s’applique qu’au motif de remplissage Gyroïde.',
	// ---
	// Vague i18n Orca main (2026-11-25, bascule aefb6b3d) — 111 chaînes absentes de
	// OrcaSlicer_fr.po : nouvelles options process (vitesses volumétriques, sous-couches
	// mixtes, stratégie de remplissage, rétraction post-paroi…) et machine/filament
	// (buses/AMS, variantes d'extrudeur, limites %1%). Les phrases déjà traduites dans le
	// .po sont reprises telles quelles (« Vitesse des parois et de la bordure de la
	// couche initiale. », « Petits périmètres »…) ; le reste est une traduction propre
	// calée sur le vocabulaire du .po. Marques et énumérations identiques en français :
	// identité (parité Orca).
	'Always retract when the nozzle leaves an outer wall, even if the travel is shorter than the travel distance threshold.\n\nWithout a retraction, a very short travel, for example to gap infill inside a thin wall, can drag the end of the outer wall inward and leave a visible dent at the seam.':
		"Toujours rétracter lorsque la buse quitte une paroi extérieure, même si le déplacement est plus court que le seuil de distance de déplacement.\n\nSans rétraction, un déplacement très court, par exemple pour rejoindre le remplissage des espaces au sein d'une paroi fine, peut tirer l'extrémité de la paroi extérieure vers l'intérieur et laisser un creux visible à la couture.",
	Crossing: 'Croisement',
	'Filament to print the brim.\n"Default" prints it with the first filament that prints the object on the first layer.':
		"Filament utilisé pour imprimer la bordure.\n« Défaut » l'imprime avec le premier filament qui imprime l'objet sur la couche initiale.",
	'How the lines of the Grid, Triangles, Tri-hexagon, Cubic, Quarter Cubic, Adaptive Cubic and Support Cubic patterns meet.\n - Default: crossing with a single line, non-crossing with multiline infill.\n - Crossing: the lines run straight across each other. With multiline infill, the lines of two walls overlap where they cross.\n - Non-crossing: the lines turn back where they meet instead of crossing, so the walls touch without overlapping.':
		'Comment les lignes des motifs Grille, Triangles, Tri-hexagone, Cubique, Quartier Cubique, Cubique adaptatif et Support Cubique se rencontrent.\n - Défaut : croisement en ligne unique, sans croisement en remplissage multiligne.\n - Croisement : les lignes se traversent tout droit. En remplissage multiligne, les lignes de deux parois se chevauchent là où elles se croisent.\n - Sans croisement : les lignes font demi-tour là où elles se rencontrent au lieu de se croiser ; les parois se touchent ainsi sans se chevaucher.',
	'Non-crossing': 'Sans croisement',
	'Retract after outer wall': 'Rétracter après la paroi extérieure',
	'Same as "Small perimeters", but for supports. This separate setting will affect the speed of support for areas <= `small_support_perimeter_threshold`. If expressed as a percentage (for example: 80%), it will be calculated on the support or support interface speed setting above.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		"Comme « Petits périmètres », mais pour les supports. Ce réglage distinct affecte la vitesse des supports pour les zones <= `small_support_perimeter_threshold`. S'il est exprimé en pourcentage (par exemple : 80 %), il est calculé à partir du réglage de vitesse des supports ou des interfaces de support ci-dessus.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.",
	'Set the speed of each feature as a volumetric flow (mm³/s) instead of a linear speed (mm/s). Each line is then printed at its feature\'s flow divided by the line\'s cross section, so lines of different width or height extrude the same flow. A volumetric speed expressed as a percentage is calculated on the filament\'s "Max volumetric speed", which still limits every speed, as "Max external volumetric speed" does for the visible features.\n\nSmall perimeter, overhang, ironing and travel speeds keep their own settings, applied over the resulting speeds. Calibration prints ignore this option.':
		"Définit la vitesse de chaque fonction en un débit volumétrique (mm³/s) au lieu d'une vitesse linéaire (mm/s). Chaque ligne est alors imprimée au débit de sa fonction divisé par la section de la ligne : des lignes de largeur ou de hauteur différentes extrudent ainsi le même débit. Une vitesse volumétrique exprimée en pourcentage est calculée sur la « Vitesse volumétrique maximale » du filament, qui limite toujours toutes les vitesses, comme le fait la « Vitesse volumétrique externe maximale » pour les fonctions visibles.\n\nLes vitesses des petits périmètres, surplombs, lissage et déplacements conservent leurs propres réglages, appliqués par-dessus les vitesses résultantes. Les impressions de calibration ignorent cette option.",
	'Sparse infill strategy': 'Stratégie de remplissage',
	'Speed of internal bridges. If the value is expressed as a percentage, it will be calculated based on the bridge_speed. Default value is 150%.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse des ponts internes. Si la valeur est exprimée en pourcentage, elle sera calculée sur la base de la vitesse des ponts. La valeur par défaut est 150 %.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'Speed of the externally visible bridge extrusions.\n\nIn addition, if Slow down for curled perimeters is disabled, it will be the print speed of overhang walls that are supported by less than 13%, whether they are part of a bridge or an overhang.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and "Max external volumetric speed".':
		"Vitesse des extrusions de ponts visibles de l'extérieur.\n\nEn outre, si l'option « Ralentir lors des périmètres courbés » est désactivée, il s'agira de la vitesse d'impression des parois en surplomb dont l'appui est inférieur à 13 %, qu'elles fassent partie d'un pont ou d'un surplomb.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et la « Vitesse volumétrique externe maximale ».",
	'The direction change at a corner is measured over a window of this length along the outline.\nThe window grows while the outline keeps curving, so a rounded corner counts as one corner once its whole turn is seen.\n0 to measure the angle at every vertex without smoothing.':
		"Le changement de direction dans un angle est mesuré sur une fenêtre de cette longueur le long du contour.\nLa fenêtre s'agrandit tant que le contour continue de courber : un angle arrondi ne compte donc que pour un seul angle une fois son virage complet pris en compte.\n0 pour mesurer l'angle à chaque sommet, sans lissage.",
	'This is the print speed for ironing lines.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		"Vitesse d'impression des lignes de lissage.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.",
	'This is the printing speed for the outer walls of parts. These are generally printed slower than inner walls for higher quality.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and "Max external volumetric speed".':
		"Vitesse d'impression des parois extérieures des pièces, généralement imprimées plus lentement que les parois intérieures pour une meilleure qualité.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et la « Vitesse volumétrique externe maximale ».",
	'This is the speed for gap infill. Gaps usually have irregular line width and should be printed more slowly.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse de remplissage des espaces. Les espaces ont généralement une largeur de ligne irrégulière et doivent être imprimés plus lentement.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for inner walls.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse des parois intérieures.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for internal solid infill, not including the top or bottom surface.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse du remplissage plein interne, sans inclure les surfaces supérieure et inférieure.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for internal sparse infill.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse de remplissage interne.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for solid top surface infill.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and, unless the surface is ironed, "Max external volumetric speed".':
		'Vitesse de remplissage de la surface supérieure pleine.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et, sauf si la surface est lissée, par la « Vitesse volumétrique externe maximale ».',
	'This is the speed for support interfaces.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse des interfaces de support.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for support.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		'Vitesse des supports.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.',
	'This is the speed for the first layer except for walls and brim, such as infill, gap fill and support.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		"Vitesse de la couche initiale, à l'exception des parois et de la bordure, comme le remplissage, le remplissage des espaces et les supports.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.",
	'This is the speed for the walls and brim of the first layer.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and, on outer walls, "Max external volumetric speed".':
		'Vitesse des parois et de la bordure de la couche initiale.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et, pour les parois extérieures, par la « Vitesse volumétrique externe maximale ».',
	'This separate setting will affect the speed of perimeters having radius <= small_perimeter_threshold (usually holes). If expressed as percentage (for example: 80%) it will be calculated on the outer wall speed setting above.\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and, on outer walls, "Max external volumetric speed".':
		"Ce paramètre séparé affectera la vitesse des périmètres ayant un rayon <= small_perimeter_threshold (généralement des trous). S'il est exprimé en pourcentage (par exemple : 80 %), il sera calculé sur le réglage de vitesse de la paroi extérieure ci-dessus.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et, pour les parois extérieures, par la « Vitesse volumétrique externe maximale ».",
	'Volumetric flow of this feature when volumetric speeds are enabled. If expressed as a percentage, it is calculated on the filament\'s "Max volumetric speed".\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and "Max external volumetric speed".':
		"Débit volumétrique de cette fonction lorsque les vitesses volumétriques sont activées. S'il est exprimé en pourcentage, il est calculé sur la « Vitesse volumétrique maximale » du filament.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et la « Vitesse volumétrique externe maximale ».",
	'Volumetric flow of this feature when volumetric speeds are enabled. If expressed as a percentage, it is calculated on the filament\'s "Max volumetric speed".\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and, on outer walls, "Max external volumetric speed".':
		"Débit volumétrique de cette fonction lorsque les vitesses volumétriques sont activées. S'il est exprimé en pourcentage, il est calculé sur la « Vitesse volumétrique maximale » du filament.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et, pour les parois extérieures, par la « Vitesse volumétrique externe maximale ».",
	'Volumetric flow of this feature when volumetric speeds are enabled. If expressed as a percentage, it is calculated on the filament\'s "Max volumetric speed".\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed" and, unless the surface is ironed, "Max external volumetric speed".':
		"Débit volumétrique de cette fonction lorsque les vitesses volumétriques sont activées. S'il est exprimé en pourcentage, il est calculé sur la « Vitesse volumétrique maximale » du filament.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament et, sauf si la surface est lissée, par la « Vitesse volumétrique externe maximale ».",
	'Volumetric flow of this feature when volumetric speeds are enabled. If expressed as a percentage, it is calculated on the filament\'s "Max volumetric speed".\n\nThe resulting speed is limited by the filament\'s "Max volumetric speed".':
		"Débit volumétrique de cette fonction lorsque les vitesses volumétriques sont activées. S'il est exprimé en pourcentage, il est calculé sur la « Vitesse volumétrique maximale » du filament.\n\nLa vitesse résultante est limitée par la « Vitesse volumétrique maximale » du filament.",
	'Volumetric speeds': 'Vitesses volumétriques',
	// Machine / filament Orca main (buses, AMS, variantes, mapping, limites machine) :
	'AMS counts per extruder.': "Nombres d'AMS par extrudeur.",
	'Allow filaments with high/low temperature to be printed together':
		"Autoriser l'impression simultanée de filaments à haute et basse température",
	'Allow filaments with high/low temperature to be printed together.':
		"Autoriser l'impression simultanée de filaments à haute et basse température.",
	'Default Nozzle Volume Type.': 'Type de volume de buse par défaut.',
	'Default Nozzle volume type for extruders in this printer.':
		'Type de volume de buse par défaut pour les extrudeurs de cette imprimante.',
	'Default extruder id to place filament.':
		"Identifiant d'extrudeur par défaut pour placer le filament.",
	'Extruder AMS count': "Nombre d'AMS par extrudeur",
	'Extruder nozzle stats': "Statistiques de buses de l'extrudeur",
	'Extruder nozzle volume type': "Type de volume de buse de l'extrudeur",
	'Extruder variant list': "Liste des variantes d'extrudeur",
	'Extruder variant list.': "Liste des variantes d'extrudeur.",
	'Filament map plus for multi nozzle': 'Carte de filaments étendue pour multi-buses',
	'Filament map to the index identified by extruder and nozzle_volume_type':
		"Carte des filaments vers l'index identifié par extrudeur et nozzle_volume_type",
	'Filament mapping mode used as plate param.':
		'Mode de mappage des filaments utilisé comme paramètre de plateau.',
	'Filament self index': 'Index propre du filament',
	'Filament self index.': 'Index propre du filament.',
	"Filament's extruder variant": "Variante d'extrudeur du filament",
	"Filament's extruder variant.": "Variante d'extrudeur du filament.",
	'Head wrap detect zone': "Zone de détection d'enroulement de la tête",
	'Lower volumetric speed limit for the visible external features: outer walls, external bridges and overhang walls, and top surfaces that are not ironed. It applies on top of the max volumetric speed. 0 means no extra limit.\n\nSet it to the highest flow at which this material still prints glossy, to keep the surface finish while the rest of the print runs at the max volumetric speed.':
		"Limite de vitesse volumétrique inférieure pour les fonctions externes visibles : parois extérieures, ponts externes et parois en surplomb, et surfaces supérieures non lissées. Elle s'applique par-dessus la vitesse volumétrique maximale. 0 signifie aucune limite supplémentaire.\n\nRéglez-la au débit le plus élevé auquel ce matériau s'imprime encore avec un aspect brillant, afin de préserver l'état de surface pendant que le reste de l'impression s'exécute à la vitesse volumétrique maximale.",
	'Map the logical extruder to physical extruder':
		"Associer l'extrudeur logique à l'extrudeur physique",
	'Map the logical extruder to physical extruder.':
		"Associe l'extrudeur logique à l'extrudeur physique.",
	'Master extruder id': "Identifiant de l'extrudeur maître",
	'Max external volumetric speed': 'Vitesse volumétrique externe maximale',
	'Maximum speed %1%': 'Vitesse maximale %1%',
	'Maximum speed of E axis': "Vitesse maximale de l'axe E",
	'Maximum speed of X axis': "Vitesse maximale de l'axe X",
	'Maximum speed of Y axis': "Vitesse maximale de l'axe Y",
	'Maximum speed of Z axis': "Vitesse maximale de l'axe Z",
	'Maximum speed of the %1% axis': "Vitesse maximale de l'axe %1%",
	'Nozzle Volume Type': 'Type de volume de buse',
	'Nozzle volume type for extruders.': 'Type de volume de buse pour les extrudeurs.',
	'Nozzle volume type per physical nozzle of an extruder.':
		"Type de volume de buse par buse physique d'un extrudeur.",
	'Physical nozzle counts per extruder, keyed by nozzle volume type.':
		'Nombre de buses physiques par extrudeur, indexé par type de volume de buse.',
	'Print extruder id': "Identifiant de l'extrudeur d'impression",
	'Print extruder id.': "Identifiant de l'extrudeur d'impression.",
	"Print's extruder variant": "Variante d'extrudeur de l'impression",
	"Print's extruder variant.": "Variante d'extrudeur de l'impression.",
	'Printer extruder id': "Identifiant de l'extrudeur de l'imprimante",
	'Printer extruder id.': "Identifiant de l'extrudeur de l'imprimante.",
	"Printer's extruder variant": "Variante d'extrudeur de l'imprimante",
	"Printer's extruder variant.": "Variante d'extrudeur de l'imprimante.",
	'Speed definition': 'Définition des vitesses',
	"This setting is only used for initial value of manual calibration of pressure advance. Bowden extruder usually has larger PA value. This setting doesn't influence normal slicing.":
		"Ce réglage sert uniquement de valeur initiale pour la calibration manuelle du Pressure Advance. Un extrudeur Bowden a généralement une valeur de PA plus élevée. Ce réglage n'influence pas la découpe normale.",
	XAxis: 'Axe X',
	YAxis: 'Axe Y',
	'extruder nozzle count': "Nombre de buses de l'extrudeur",
	'filament mapping mode': 'Mode de mappage des filaments',
	// Marques, firmwares, formats — identiques en français (parité Orca) :
	'3DPrinterOS': '3DPrinterOS',
	AstroBox: 'AstroBox',
	'BTT TT': 'BTT TT',
	ColPic: 'ColPic',
	CrealityPrint: 'CrealityPrint',
	Duet: 'Duet',
	ESP3D: 'ESP3D',
	'Elegoo Link': 'Elegoo Link',
	FlashAir: 'FlashAir',
	Flashforge: 'Flashforge',
	JPG: 'JPG',
	MKS: 'MKS',
	'Marlin 2': 'Marlin 2',
	'Moonraker (Klipper)': 'Moonraker (Klipper)',
	Obico: 'Obico',
	'Octo/Klipper': 'Octo/Klipper',
	PNG: 'PNG',
	PrusaConnect: 'PrusaConnect',
	PrusaLink: 'PrusaLink',
	QOI: 'QOI',
	RepRapFirmware: 'RepRapFirmware',
	Repetier: 'Repetier',
	SimplyPrint: 'SimplyPrint',
	UltiMaker: 'UltiMaker',
	// Énumérations numériques et unités identiques en français :
	'0': '0',
	'1': '1',
	'2': '2',
	'3': '3',
	'4': '4',
	'5': '5',
	'1 mm': '1 mm',
	'2 mm': '2 mm',
	'5 mm': '5 mm',
	'10 mm': '10 mm'
};
