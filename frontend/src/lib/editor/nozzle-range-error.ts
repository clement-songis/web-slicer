// Lot 47 (audit 45 H2D-a) : refus du tranchage quand un trajet sort de la zone
// atteignable d'une buse (GCodeProcessorResult::gcode_check_result, bits
// multi-buses 0 / 1). Le moteur renvoie le texte de la GUI d'Orca
// (GLCanvas3D::_set_warning_notification, MultiExtruderPrintableError /
// MultiExtruderHeightOutside) précédé de « tranchage impossible : » ; on le
// restitue comme Orca, traduit avec les msgstr d'OrcaSlicer_fr.po.
import { tr } from '$lib/i18n';

const LINE =
	/^(Filaments?) (.+?) (?:is|are) placed in the (.+?), but the generated G-code path exceeds the printable (range|height) of the (.+?)\.$/;
const GENERIC =
	'Found G-code in unprintable area of multi-extruder printers after slicing. Please make sure the 3mf file can be successfully sliced in the latest Orca Slicer.';

function nozzleName(name: string): string {
	if (name === 'left nozzle') return tr('left nozzle');
	if (name === 'right nozzle') return tr('right nozzle');
	const tool = /^Tool (\d+)$/.exec(name);
	return tool ? tr('Tool {n}', { n: tool[1] }) : name;
}

/** Texte d'Orca (une ligne par buse, + « Open wiki… » pour la hauteur) traduit,
 *  ou null si `message` n'est pas ce refus. */
export function nozzleRangeError(message: string): string | null {
	const text = message.replace(/^\s*tranchage impossible\s*:\s*/i, '').trim();
	if (text.startsWith(GENERIC)) return tr(GENERIC);
	// La sortie du worker peut réduire les sauts de ligne à des espaces.
	const lines = text
		.replace(/\.\s+(?=Filaments? |Open wiki)/g, '.\n')
		.split('\n')
		.map((l) => l.trim())
		.filter(Boolean);
	if (!lines.length || !LINE.test(lines[0])) return null;
	const out: string[] = [];
	for (const line of lines) {
		if (line === 'Open wiki for more information.') {
			out.push(tr('Open wiki for more information.'));
			continue;
		}
		const m = LINE.exec(line);
		if (!m) return null;
		const [, word, filaments, , kind, nozzle] = m;
		const params = { filaments, nozzle: nozzleName(nozzle) };
		const plural = word === 'Filaments';
		out.push(
			kind === 'range'
				? plural
					? tr(
							'Filaments {filaments} are placed in the {nozzle}, but the generated G-code path exceeds the printable range of the {nozzle}.',
							params
						)
					: tr(
							'Filament {filaments} is placed in the {nozzle}, but the generated G-code path exceeds the printable range of the {nozzle}.',
							params
						)
				: plural
					? tr(
							'Filaments {filaments} are placed in the {nozzle}, but the generated G-code path exceeds the printable height of the {nozzle}.',
							params
						)
					: tr(
							'Filament {filaments} is placed in the {nozzle}, but the generated G-code path exceeds the printable height of the {nozzle}.',
							params
						)
		);
	}
	return out.join('\n');
}

const TEMPLATES = [
	'Filament {filaments} is placed in the {nozzle}, but the generated G-code path exceeds the printable range of the {nozzle}.',
	'Filaments {filaments} are placed in the {nozzle}, but the generated G-code path exceeds the printable range of the {nozzle}.',
	'Filament {filaments} is placed in the {nozzle}, but the generated G-code path exceeds the printable height of the {nozzle}.',
	'Filaments {filaments} are placed in the {nozzle}, but the generated G-code path exceeds the printable height of the {nozzle}.'
];

function templateRegex(template: string): RegExp {
	const escaped = template.replace(/[.*+?^$()|[\]\\]/g, '\\$&');
	return new RegExp('^' + escaped.replace(/\{(?:filaments|nozzle)\}/g, '.+?') + '$');
}

/** Audit 46 GCHK-a : trajet hors du plateau (`gcode_check_result` bit 2,
 *  imprimantes mono-buse comprises, tour de purge comprise) — erreur
 *  SLICING_ERROR de la GUI d'Orca (EWarning::ToolpathOutside,
 *  GLCanvas3D.cpp:10102) ; Imprimer / Envoyer désactivés
 *  (`PartPlate::is_slice_result_ready_for_print`). */
export const PLATE_BOUNDARY_ERROR = 'A G-code path goes beyond the plate boundaries.';
/** Bit 3 : trajet plus haut que `printable_height` (GLCanvas3D.cpp:10101). */
export const PLATE_HEIGHT_ERROR = 'A G-code path goes beyond the max print height.';
const PLATE_BOUNDARY_BIT = 1 << 2;
const PLATE_HEIGHT_BIT = 1 << 3;

function checkParts(raw: unknown): { plate: boolean; height: boolean; rest: string } | null {
	if (!raw || typeof raw !== 'object') return null;
	const { message, error_code: code } = raw as { message?: unknown; error_code?: unknown };
	const text =
		typeof message === 'string'
			? message.replace(/^\s*tranchage impossible\s*:\s*/i, '').trim()
			: '';
	const bits = typeof code === 'number' && Number.isFinite(code) ? code : 0;
	const plate = text.includes(PLATE_BOUNDARY_ERROR) || (bits & PLATE_BOUNDARY_BIT) !== 0;
	const height = text.includes(PLATE_HEIGHT_ERROR) || (bits & PLATE_HEIGHT_BIT) !== 0;
	const rest = text
		.split(PLATE_BOUNDARY_ERROR)
		.join('\n')
		.split(PLATE_HEIGHT_ERROR)
		.join('\n')
		.split('\n')
		.map((l) => l.trim())
		.filter(Boolean)
		.join('\n');
	if (!plate && !height && !rest) return null;
	return { plate, height, rest };
}

/** Lot 47c : comme l'interface d'Orca, le tranchage réussit et l'aperçu reste
 *  affiché ; le serveur remonte le contrôle dans `stats.gcode_check`
 *  (`{ error_code, message, issues }`) et refuse export / envoi (409).
 *  Renvoie le texte d'Orca traduit (une ligne par problème : trajet hors du
 *  plateau, puis buses), ou null si aucun problème. */
export function gcodeCheckError(raw: unknown): string | null {
	const parts = checkParts(raw);
	if (!parts) return null;
	const out: string[] = [];
	if (parts.plate) out.push(tr(PLATE_BOUNDARY_ERROR));
	if (parts.height) out.push(tr(PLATE_HEIGHT_ERROR));
	if (parts.rest) out.push(nozzleRangeError(parts.rest) ?? parts.rest);
	return out.join('\n');
}

/** Nature du refus pour le bandeau : `plate-boundary` (trajet hors du
 *  plateau, bit 2), `plate-height` (au-dessus de la hauteur imprimable,
 *  bit 3) ou `nozzle-range` (zone / hauteur d'une buse, bits 0-1). */
export function gcodeCheckKind(
	raw: unknown
): 'plate-boundary' | 'plate-height' | 'nozzle-range' | null {
	const parts = checkParts(raw);
	if (!parts) return null;
	return parts.plate ? 'plate-boundary' : parts.height ? 'plate-height' : 'nozzle-range';
}

/** Vrai si l'erreur de tranchage déjà rendue lisible (`friendlySliceError`,
 *  langue courante ou anglais) est ce refus : impression / export bloqués. */
export function isNozzleRangeError(message: string | null | undefined): boolean {
	if (!message) return false;
	if (message === GENERIC || message === tr(GENERIC)) return true;
	const first = message.split('\n')[0].trim();
	return TEMPLATES.some((t) => templateRegex(t).test(first) || templateRegex(tr(t)).test(first));
}
