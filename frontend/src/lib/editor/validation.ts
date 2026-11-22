// Validation avant tranchage (phase 29) : POST /api/projects/{id}/validate,
// équivalent de Print::validate qu'Orca rejoue à chaque modification
// (Plater::priv::update_background_process). Fonctions pures : traduction des
// messages d'Orca par son catalogue officiel, notifications de la vue
// (NotificationManager : ValidateError / ValidateWarning, lien « Jump to »),
// objets fautifs et option à mettre en évidence. La page fait les appels.
import type { ValidateResponse } from '../../generated/api/ValidateResponse';
import type { ValidationIssue } from '../../generated/api/ValidationIssue';
import type { SliceRequest } from '../../generated/api/SliceRequest';
import { PARAMS } from '../../generated/params';
import { UI_LAYOUT, type PresetKind } from '../../generated/ui-layout';
import { api } from '$lib/api/client';
import { tr } from '$lib/i18n';
import { nozzleRangeError } from './nozzle-range-error';

export type { ValidateResponse, ValidationIssue };

/** Valide le projet persisté (même corps que /slice ; aucun job créé). */
export const validateProject = (projectId: string, body: SliceRequest) =>
	api.post<ValidateResponse>('/projects/' + projectId + '/validate', body);

/** Catalogue « msgid anglais → msgstr français » (scripts/gen-validation-i18n.mjs). */
export type OrcaCatalog = Readonly<Record<string, string>>;

let catalogLoad: Promise<OrcaCatalog> | null = null;
/** Catalogue FR chargé à la demande (hors chunk initial de l'éditeur). */
export function loadOrcaCatalog(): Promise<OrcaCatalog> {
	catalogLoad ??= import('./validation-messages.fr.json').then(
		(m) => (m.default ?? m) as OrcaCatalog
	);
	return catalogLoad;
}

// Messages du moteur / du worker absents du .po d'Orca (préfixes de causes).
const EXTRA: OrcaCatalog = {
	'Failed loading the input model': 'Échec du chargement du modèle',
	'Failed loading the input model:': 'Échec du chargement du modèle :',
	// Audit 46 GCHK-a : notice `gcode_check` « trajet hors du plateau »
	// (msgstr d'OrcaSlicer_fr.po:5901, absent du catalogue généré).
	'A G-code path goes beyond the plate boundaries.':
		'Un chemin du G-code va au-delà de la limite du plateau.',
	'A G-code path goes beyond the max print height.':
		'Un chemin du G-code dépasse la hauteur d’impression maximale.'
};

// Audit 47 I18N-b : noms de plaque passés à L() par Print.cpp:1771
// (`L(bed_type_name)`, clés de s_keys_map_BedType) — msgstr actifs
// d'OrcaSlicer_fr.po (10922, 10150, 10156, 10933) ; « High Temp Plate »
// (obsolète dans le .po), « Supertack Plate », « Default Plate » restent en anglais.
const BED_TYPE_NAMES: OrcaCatalog = {
	'Cool Plate': 'Cool Plate/Plaque PLA',
	'Engineering Plate': 'Plaque Engineering',
	'Textured PEI Plate': 'Plaque PEI texturée',
	'Textured Cool Plate': 'Plaque Cool plate texturée'
};
// Gabarits dont un paramètre est lui-même traduit (indice 0-based → table).
const TRANSLATED_ARGS: Readonly<Record<string, Readonly<Record<number, OrcaCatalog>>>> = {
	'Plate %d: %s does not support filament %s': { 1: BED_TYPE_NAMES },
	// Plater.cpp:18313 : err.params[1] = L(bed_type_name) (déjà traduit).
	"Plate %d: %s is not suggested to be used to print filament %s (%s). If you still want to do this print job, please set this filament's bed temperature to non-zero.":
		{ 1: BED_TYPE_NAMES }
};
function translateArgs(id: string, args: string[]): string[] {
	const map = TRANSLATED_ARGS[id];
	if (!map) return args;
	return args.map((a, i) => map[i]?.[a] ?? a);
}

// Spécificateurs de format : boost (%1%) et printf (%s, %d, %.2f, %ld…).
const SPEC = /%(?:\d+%|[-+ #0]*\d*(?:\.\d+)?(?:ll|l|z|h)?[sdufgixXc])/g;
const escapeRe = (s: string) => s.replace(/[.*+?^$()|[\]\\{}]/g, '\\$&');

interface Template {
	id: string;
	tr: string;
	full: RegExp;
	/** Recherche dans un message composé (gabarit sans paramètre aux bords). */
	inner: RegExp | null;
	literal: number;
}

const compiled = new WeakMap<object, Template[]>();
function templates(catalog: OrcaCatalog): Template[] {
	const hit = compiled.get(catalog);
	if (hit) return hit;
	const out: Template[] = [];
	for (const [rawId, rawTr] of Object.entries({ ...EXTRA, ...catalog })) {
		const id = rawId.trim();
		const tr = rawTr.trim();
		if (!id) continue;
		const parts = id.split(SPEC);
		const body = parts.map(escapeRe).join('(.+?)');
		const literal = parts.join('').length;
		const edgeParam = SPEC.test(id) && (/^%/.test(id) || /%[a-zA-Z]?$|%\d+%$/.test(id));
		SPEC.lastIndex = 0;
		out.push({
			id,
			tr,
			full: new RegExp('^' + body + '$', 's'),
			// Fragments : assez longs pour ne pas traduire un mot isolé.
			inner: !edgeParam && literal >= 16 ? new RegExp(body, 'gs') : null,
			literal
		});
	}
	out.sort((a, b) => b.literal - a.literal);
	compiled.set(catalog, out);
	return out;
}

/** Remplace les spécificateurs du msgstr par les valeurs capturées. */
export function fillTemplate(tr: string, args: string[]): string {
	let seq = 0;
	return tr.replace(SPEC, (spec) => {
		const pos = /^%(\d+)%$/.exec(spec);
		const value = pos ? args[Number(pos[1]) - 1] : args[seq++];
		return value ?? spec;
	});
}

/**
 * Traduit un message d'Orca (anglais, déjà formaté par le moteur) : msgid exact,
 * gabarit complet (« %1% is too close to others… » → nom capturé), puis
 * fragments d'un message composé (« objet » + « is too close to exclusion
 * area… »). Un message inconnu (ou déjà français) est rendu tel quel.
 */
export function translateOrcaMessage(message: string, catalog: OrcaCatalog | null): string {
	const msg = message.trim();
	if (!catalog || !msg) return msg;
	const direct = catalog[msg] ?? catalog[msg + '\n'] ?? EXTRA[msg];
	if (direct) return direct.trim();
	const list = templates(catalog);
	for (const t of list) {
		const m = t.full.exec(msg);
		if (m) return fillTemplate(t.tr, translateArgs(t.id, m.slice(1)));
	}
	let out = msg;
	for (const t of list) {
		if (!t.inner) continue;
		t.inner.lastIndex = 0;
		out = out.replace(t.inner, (...m) => {
			const groups = m.slice(1, -2).filter((g) => typeof g === 'string') as string[];
			return fillTemplate(t.tr, translateArgs(t.id, groups));
		});
	}
	return out;
}

/** Type de preset d'une option (Orca resolve_validation_option_type : imprimante, filament, sinon impression). */
export function optionKind(key: string): PresetKind | null {
	const kinds = new Set<PresetKind>();
	for (const page of UI_LAYOUT)
		for (const section of page.sections) if (section.options.includes(key)) kinds.add(page.kind);
	if (kinds.has('machine')) return 'machine';
	if (kinds.has('filament')) return 'filament';
	if (kinds.has('process')) return 'process';
	return null;
}

/** Page de réglages (titre Orca) qui contient l'option, dans le type donné. */
export function optionPage(key: string, kind: PresetKind): string | null {
	for (const page of UI_LAYOUT)
		if (page.kind === kind && page.sections.some((s) => s.options.includes(key))) return page.title;
	return null;
}

export type NoticeLevel = 'error' | 'warning';
export type NoticeAction = 'arrange';

/** Une notification de la vue (carte « Erreur » / « Avertissement » d'Orca). */
export interface ValidationNotice {
	id: string;
	level: NoticeLevel;
	kind: ValidationIssue['kind'] | 'config_warning';
	/** Plateau concerné (0-based) ; null = global. */
	plateIndex: number | null;
	text: string;
	/** Objet vivant de la scène désigné par Orca (« Jump to [objet] »). */
	objectId?: string;
	objectName?: string;
	/** Option en cause et son type de preset (« Jump to (opt_key) »). */
	optKey?: string;
	optKind?: PresetKind;
	/** Libellé anglais de l'option (registre), traduit à l'affichage. */
	optLabel?: string;
	action?: NoticeAction;
}

export interface NoticeContext {
	catalog: OrcaCatalog | null;
	/** Id persisté (scene.objects[].id) → id vivant ; undefined = objet absent. */
	liveId?: (persistedId: string) => string | undefined;
	/** Nombre de plateaux du projet (préfixe « Plateau N » si plusieurs). */
	plateCount?: number;
}

/** Première lettre en capitale (messages du serveur : « plateau vide… »). */
const sentence = (s: string) => (s ? s[0].toLocaleUpperCase('fr') + s.slice(1) : s);

function issueAction(kind: ValidationIssue['kind']): NoticeAction | undefined {
	return kind === 'object_collision_in_seq_print' ||
		kind === 'object_collision_in_layer_print' ||
		kind === 'object_outside_plate'
		? 'arrange'
		: undefined;
}

/**
 * Notifications d'une réponse de validation : erreurs d'abord, puis
 * avertissements, puis avertissements de configuration (racine). Le plateau
 * vide n'est pas notifié (Orca grise seulement « Trancher »).
 */
export function buildNotices(resp: ValidateResponse, ctx: NoticeContext): ValidationNotice[] {
	const multi = (ctx.plateCount ?? resp.plates.length) > 1;
	const out: ValidationNotice[] = [];
	const seen = new Set<string>();
	const push = (n: ValidationNotice) => {
		const key = [n.level, n.plateIndex, n.text, n.objectId, n.optKey].join('|');
		if (seen.has(key)) return;
		seen.add(key);
		out.push(n);
	};
	for (const level of ['error', 'warning'] as const) {
		for (const plate of resp.plates) {
			const plateIndex = Number(plate.plate_index);
			const list = level === 'error' ? plate.errors : plate.warnings;
			list.forEach((issue, i) => {
				if (issue.kind === 'empty_plate') return;
				const objectId = issue.object_id
					? ctx.liveId
						? ctx.liveId(issue.object_id)
						: issue.object_id
					: undefined;
				const optKind = issue.opt_key ? (optionKind(issue.opt_key) ?? undefined) : undefined;
				// Lot 47c : trajets hors zone d'une buse (texte de la GUI d'Orca,
				// une ligne par buse) traduit comme la notification d'Orca.
				const translated =
					(issue.kind === 'gcode_check' ? nozzleRangeError(issue.message) : null) ??
					translateOrcaMessage(issue.message, ctx.catalog);
				// Le message d'Orca commence souvent par le nom de l'objet : casse gardée.
				const text =
					issue.object_name && translated.startsWith(issue.object_name)
						? translated
						: sentence(translated);
				push({
					id: level + ':' + plateIndex + ':' + i + ':' + issue.kind,
					level,
					kind: issue.kind,
					plateIndex,
					text: multi ? tr('Plate {n}: {text}', { n: plateIndex + 1, text }) : text,
					objectId,
					objectName: issue.object_name,
					optKey: issue.opt_key && optKind ? issue.opt_key : undefined,
					optKind,
					optLabel: issue.opt_key ? PARAMS[issue.opt_key]?.label : undefined,
					action: issueAction(issue.kind)
				});
			});
		}
	}
	resp.warnings.forEach((w, i) => {
		const optKind = PARAMS[w.key] ? (optionKind(w.key) ?? undefined) : undefined;
		push({
			id: 'config:' + i + ':' + w.key,
			level: 'warning',
			kind: 'config_warning',
			plateIndex: null,
			text: sentence(translateOrcaMessage(w.message, ctx.catalog)),
			optKey: optKind ? w.key : undefined,
			optKind,
			optLabel: PARAMS[w.key]?.label
		});
	});
	return out;
}

/** Niveau le plus grave par objet vivant (surlignage vue 3D et liste). */
export function issuesByObject(
	notices: readonly ValidationNotice[]
): Record<string, { level: NoticeLevel; text: string }> {
	const out: Record<string, { level: NoticeLevel; text: string }> = {};
	for (const n of notices) {
		if (!n.objectId) continue;
		const prev = out[n.objectId];
		if (!prev || (prev.level === 'warning' && n.level === 'error'))
			out[n.objectId] = { level: n.level, text: n.text };
	}
	return out;
}

/** Erreurs bloquantes du plateau (Orca grise « Trancher le plateau »).
 *  `gcode_check` (lot 47c) bloque l'impression / l'export, pas le tranchage. */
export function plateBlocked(resp: ValidateResponse | null, plateIndex: number): boolean {
	const plate = resp?.plates.find((p) => Number(p.plate_index) === plateIndex);
	return !!plate && plate.errors.some((e) => e.kind !== 'empty_plate' && e.kind !== 'gcode_check');
}

/** Résumé court (« 2 erreurs · 1 avertissement »). */
export function noticeSummary(notices: readonly ValidationNotice[]): string {
	const errors = notices.filter((n) => n.level === 'error').length;
	const warnings = notices.length - errors;
	const part = (n: number, one: string, many: string) => (n ? tr(n > 1 ? many : one, { n }) : '');
	return [part(errors, '{n} error', '{n} errors'), part(warnings, '{n} warning', '{n} warnings')]
		.filter(Boolean)
		.join(' · ');
}

/**
 * Cause d'un échec de conversion (ModelResponse.conversion_error, 422
 * conversion_failed details.cause, événement model.conversion_failed),
 * traduite, en phrase courte affichable après « Conversion impossible : ».
 */
export function conversionCause(
	cause: string | null | undefined,
	catalog: OrcaCatalog | null
): string {
	const raw = (cause ?? '').trim();
	if (!raw) return tr('the engine could not extract a mesh from this file');
	return translateOrcaMessage(raw, catalog);
}
