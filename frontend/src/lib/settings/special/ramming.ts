// Paramètres de ramming (filament_ramming_parameters), édités chez Orca par
// RammingDialog (WipeTowerDialog.cpp + RammingChart.cpp, PrusaSlicer). Chaîne :
//   « <largeur %> <espacement %> <débit mm³/s par pas de 0,25 s>… | <x y>… »
// — avant « | » : largeur et espacement des lignes de ramming puis un débit
// volumétrique par pas de 0,25 s (durée totale = nombre de pas × 0,25 s) ;
// après : les points de contrôle de la courbe (temps s, débit mm³/s).
//
// Parité : la courbe est la spline cubique du Chart d'Orca (dérivée nulle aux
// extrémités), échantillonnée comme lui sur la grille de pixels du graphique à
// 100 % (rectangle 430 × 310 px, débit 0–60 mm³/s) puis moyennée par pas de
// 0,25 s ; nombres écrits comme un flux C++ (6 chiffres significatifs).
// Chaîne exacte conservée : sans modification de la courbe, la chaîne d'origine
// est rendue telle quelle (seuls largeur / espacement sont réécrits s'ils
// changent).
import { PARAMS } from '../../../generated/params';

/** Pas d'échantillonnage du débit (s). */
export const RAMMING_STEP = 0.25;
/** Durée maximale (SpinInput « Time » d'Orca : 0–5000 ms par 250). */
export const RAMMING_MAX_TIME = 5;
/** Débit maximal affiché (visible_area : 0–60 mm³/s). */
export const RAMMING_MAX_SPEED = 60;
/** Bornes de largeur / espacement (SpinInput 10–300 %). */
export const RAMMING_PERCENT_MIN = 10;
export const RAMMING_PERCENT_MAX = 300;

// Géométrie du Chart d'Orca à 100 % (em_unit = 10) : wxRect(10,10,480,360),
// légende 50 px → m_rect = (50, 0, 430, 310).
const RECT_LEFT = 50;
const RECT_WIDTH = 430;
const RECT_HEIGHT = 310;
const RECT_RIGHT = RECT_LEFT + RECT_WIDTH - 1; // wxRect::GetRight
const RECT_BOTTOM = RECT_HEIGHT - 1; // wxRect::GetBottom (top = 0)

export type RammingPoint = [number, number];

export interface Ramming {
	/** Largeur des lignes de ramming (% de la largeur d'extrusion). */
	lineWidth: number;
	/** Espacement des lignes de ramming (%). */
	spacing: number;
	/** Débit volumétrique (mm³/s) par pas de 0,25 s. */
	speeds: number[];
	/** Points de contrôle (temps s, débit mm³/s). */
	points: RammingPoint[];
}

/** Valeur par défaut d'Orca (PrintConfig, filament_ramming_parameters). */
export const DEFAULT_RAMMING: string = (() => {
	const d = PARAMS['filament_ramming_parameters']?.default;
	return Array.isArray(d) && typeof d[0] === 'string'
		? d[0]
		: '120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0| 0.05 6.6 0.45 6.8 0.95 7.8 1.45 8.3 1.95 9.7 2.45 10 2.95 7.6 3.45 7.6 3.95 7.6 4.45 7.6 4.95 7.6';
})();

/** Abscisses des points par défaut d'Orca (un point toutes les 0,5 s). */
const DEFAULT_XS = [0.05, 0.45, 0.95, 1.45, 1.95, 2.45, 2.95, 3.45, 3.95, 4.45, 4.95];

function numbers(text: string): number[] {
	const out: number[] = [];
	for (const tok of text.trim().split(/\s+/)) {
		if (!tok) continue;
		const n = Number(tok);
		if (!Number.isFinite(n)) break; // le flux C++ s'arrête au premier jeton illisible
		out.push(n);
	}
	return out;
}

/**
 * Lit la chaîne comme RammingPanel (flux : largeur, espacement, débits jusqu'au
 * « | », puis paires x y). Une chaîne sans points (ancien format) reçoit des
 * points posés sur les débits, aux abscisses par défaut d'Orca.
 */
export function parseRamming(raw: string | null | undefined): Ramming {
	const text = typeof raw === 'string' && raw.trim() ? raw : DEFAULT_RAMMING;
	const bar = text.indexOf('|');
	const head = numbers(bar >= 0 ? text.slice(0, bar) : text);
	const tail = bar >= 0 ? numbers(text.slice(bar + 1)) : [];
	const lineWidth = head[0] ?? 100;
	const spacing = head[1] ?? 100;
	const speeds = head.slice(2);
	const points: RammingPoint[] = [];
	for (let i = 0; i + 1 < tail.length; i += 2) points.push([tail[i], tail[i + 1]]);
	return {
		lineWidth,
		spacing,
		speeds,
		points: points.length ? points : seedPoints(speeds)
	};
}

/** Points de contrôle déduits des débits (chaîne sans partie « | »). */
export function seedPoints(speeds: readonly number[]): RammingPoint[] {
	if (!speeds.length) return DEFAULT_XS.map((x) => [x, 0]);
	return DEFAULT_XS.map((x) => {
		const i = Math.min(speeds.length - 1, Math.floor(x / RAMMING_STEP));
		return [x, speeds[i]];
	});
}

/** Durée totale (s) = nombre de pas × 0,25. */
export function rammingTime(r: Pick<Ramming, 'speeds'>): number {
	return r.speeds.length * RAMMING_STEP;
}

/** Durée arrondie au pas inférieur et bornée (Chart::set_xy_range). */
export function clampTime(seconds: number): number {
	if (!Number.isFinite(seconds) || seconds < 0) return 0;
	return Math.min(RAMMING_MAX_TIME, Math.floor(seconds / RAMMING_STEP + 1e-9) * RAMMING_STEP);
}

/** Écriture d'un nombre comme `std::ostream << float` (précision 6, %g). */
export function formatStreamNumber(n: number): string {
	if (!Number.isFinite(n)) return '0';
	const f = Math.fround(n);
	if (f === 0) return '0';
	const exp = Math.floor(Math.log10(Math.abs(Number(f.toPrecision(6)))));
	if (exp < -4 || exp >= 6) {
		const [m, e] = f.toExponential(5).split('e');
		const mant = m.includes('.') ? m.replace(/0+$/, '').replace(/\.$/, '') : m;
		const ev = Number(e);
		return `${mant}e${ev < 0 ? '-' : '+'}${String(Math.abs(ev)).padStart(2, '0')}`;
	}
	const fixed = f.toFixed(Math.max(0, 5 - exp));
	return fixed.includes('.') ? fixed.replace(/0+$/, '').replace(/\.$/, '') : fixed;
}

interface Curve {
	/** y écran (entier) par colonne de pixels, comme m_line_to_draw. */
	line: number[];
	/** Volume total (mm³), comme m_total_volume. */
	volume: number;
}

/** Spline du Chart d'Orca (recalculate_line) sur la grille de pixels. */
function computeCurve(points: readonly RammingPoint[], time: number): Curve {
	const line: number[] = [];
	let volume = 0;
	if (time <= 0) return { line, volume };
	const sx = RECT_WIDTH / time;
	const sy = RECT_HEIGHT / RAMMING_MAX_SPEED;
	const pts: { x: number; y: number }[] = [];
	for (const [mx, my] of points) {
		// math_to_screen : troncature wxPoint (vers zéro), axe y inversé.
		const p = { x: Math.trunc(mx * sx) + RECT_LEFT, y: -Math.trunc(my * sy) + RECT_BOTTOM };
		pts.push(p);
		if (pts.length > 1 && pts[pts.length - 1].x === pts[pts.length - 2].x) pts.pop();
		if (pts.length > 1 && pts[pts.length - 1].x > RECT_RIGHT) {
			pts.pop();
			break;
		}
	}
	if (pts.length <= 1) return { line, volume };
	pts.sort((a, b) => a.x - b.x);
	const f = Math.fround;
	const N = pts.length - 1;
	const diag = new Array<number>(N + 1).fill(2);
	const mu = new Array<number>(N + 1).fill(0);
	const lambda = new Array<number>(N + 1).fill(0);
	const h = new Array<number>(N + 1).fill(0);
	const rhs = new Array<number>(N + 1).fill(0);
	for (let i = 1; i <= N; i++) h[i] = pts[i].x - pts[i - 1].x;
	for (let i = 1; i <= N - 1; i++) {
		mu[i] = f(h[i] / (h[i] + h[i + 1]));
		lambda[i] = f(1 - mu[i]);
		rhs[i] = f(
			6 *
				(f(pts[i + 1].y - pts[i].y) / (h[i + 1] * (pts[i + 1].x - pts[i - 1].x)) -
					f(pts[i].y - pts[i - 1].y) / (h[i] * (pts[i + 1].x - pts[i - 1].x)))
		);
	}
	lambda[0] = 1;
	mu[N] = 1;
	rhs[0] = f((6 / h[1]) * (f(pts[0].y - pts[1].y) / (pts[0].x - pts[1].x)));
	rhs[N] = f((6 / h[N]) * (0 - f(pts[N - 1].y - pts[N].y) / (pts[N - 1].x - pts[N].x)));
	for (let i = 1; i <= N; i++) {
		const multiple = f(mu[i] / diag[i - 1]);
		diag[i] = f(diag[i] - multiple * lambda[i - 1]);
		rhs[i] = f(rhs[i] - multiple * rhs[i - 1]);
	}
	rhs[N] = f(rhs[N] / diag[N]);
	for (let i = N - 1; i >= 0; i--) rhs[i] = f((rhs[i] - lambda[i] * rhs[i + 1]) / diag[i]);

	let i = 1;
	let y: number;
	const top = 0;
	for (let x = RECT_LEFT; x <= RECT_RIGHT; x++) {
		if (i < pts.length - 1 && pts[i].x < x) i++;
		if (pts[0].x > x) y = pts[0].y;
		else if (pts[N].x < x) y = pts[N].y;
		else
			y = f(
				(rhs[i - 1] * Math.pow(pts[i].x - x, 3) + rhs[i] * Math.pow(x - pts[i - 1].x, 3)) /
					(6 * h[i]) +
					((pts[i - 1].y - (rhs[i - 1] * h[i] * h[i]) / 6) * (pts[i].x - x)) / h[i] +
					((pts[i].y - (rhs[i] * h[i] * h[i]) / 6) * (x - pts[i - 1].x)) / h[i]
			);
		let v = Math.trunc(y); // vector<int>::push_back(float)
		v = Math.max(v, top - 1);
		v = Math.min(v, RECT_BOTTOM - 1);
		line.push(v);
		volume += (RECT_BOTTOM - v) * (time / RECT_WIDTH) * (RAMMING_MAX_SPEED / RECT_HEIGHT);
	}
	return { line, volume: f(volume) };
}

/** Débit en mm³/s d'un y écran (screen_to_math). */
function screenToSpeed(y: number): number {
	return ((RECT_BOTTOM - y) * RAMMING_MAX_SPEED) / RECT_HEIGHT;
}

/**
 * Débits échantillonnés par pas de 0,25 s (Chart::get_ramming_speed) pour une
 * durée donnée. `null` si la courbe n'a pas au moins deux points visibles
 * (Orca n'a alors pas de courbe : les débits existants sont conservés).
 */
export function sampleSpeeds(points: readonly RammingPoint[], time: number): number[] | null {
	const { line } = computeCurve(points, time);
	const n = Math.round(time / RAMMING_STEP);
	if (n <= 0) return [];
	if (line.length < 2) return null;
	const dx = Math.floor((line.length - 1) / n);
	const out: number[] = [];
	for (let j = 0; j < n; j++) {
		const left = Math.fround(screenToSpeed(line[j * dx]));
		const right = Math.fround(screenToSpeed(line[(j + 1) * dx]));
		out.push(Math.fround((left + right) / 2));
	}
	return out;
}

/** Volume total (mm³) de la courbe, affiché arrondi comme le SpinInput d'Orca. */
export function rammingVolume(points: readonly RammingPoint[], time: number): number {
	return computeCurve(points, time).volume;
}

/** Courbe continue (pour le tracé) : débit en mm³/s par colonne. */
export function curveSamples(
	points: readonly RammingPoint[],
	time: number
): { t: number; speed: number }[] {
	const { line } = computeCurve(points, time);
	return line.map((y, k) => ({ t: (k * time) / RECT_WIDTH, speed: screenToSpeed(y) }));
}

/** Écrit la chaîne Orca (RammingPanel::get_parameters). */
export function serializeRamming(r: Ramming): string {
	let s = `${formatStreamNumber(Math.round(r.lineWidth))} ${formatStreamNumber(Math.round(r.spacing))}`;
	for (const v of r.speeds) s += ` ${formatStreamNumber(v)}`;
	s += '|';
	for (const [x, y] of r.points) s += ` ${formatStreamNumber(x)} ${formatStreamNumber(y)}`;
	return s;
}

/** Remplace seulement largeur et espacement dans la chaîne d'origine. */
export function replaceLineParams(raw: string, lineWidth: number, spacing: number): string {
	const m = /^(\s*)(\S+)(\s+)(\S+)/.exec(raw);
	if (!m) return raw;
	return (
		m[1] +
		formatStreamNumber(Math.round(lineWidth)) +
		m[3] +
		formatStreamNumber(Math.round(spacing)) +
		raw.slice(m[0].length)
	);
}

/** Édition en cours : état d'origine + modifications. */
export interface RammingEdit {
	original: string;
	lineWidth: number;
	spacing: number;
	time: number;
	points: RammingPoint[];
	/** Courbe ou durée modifiée (les débits doivent être recalculés). */
	curveChanged: boolean;
}

export function startEdit(raw: string | null | undefined): RammingEdit {
	const original = typeof raw === 'string' && raw.trim() ? raw : DEFAULT_RAMMING;
	const r = parseRamming(original);
	return {
		original,
		lineWidth: r.lineWidth,
		spacing: r.spacing,
		time: rammingTime(r),
		points: r.points.map(([x, y]) => [x, y] as RammingPoint),
		curveChanged: false
	};
}

/** Borne un pourcentage (10–300). */
export function clampPercent(v: number): number {
	if (!Number.isFinite(v)) return 100;
	return Math.min(RAMMING_PERCENT_MAX, Math.max(RAMMING_PERCENT_MIN, Math.round(v)));
}

/** Débit borné à l'aire du graphique (0–60 mm³/s). */
export function clampSpeed(v: number): number {
	if (!Number.isFinite(v)) return 0;
	return Math.min(RAMMING_MAX_SPEED, Math.max(0, v));
}

/**
 * Déplace un point verticalement (x fixe, comme Orca) ; `all` = débit
 * constant (Ctrl chez Orca : tous les points suivent le même décalage).
 */
export function movePoint(
	edit: RammingEdit,
	index: number,
	speed: number,
	all = false
): RammingEdit {
	const target = clampSpeed(speed);
	const delta = target - (edit.points[index]?.[1] ?? 0);
	const points = edit.points.map(([x, y], i): RammingPoint => {
		if (all) return [x, clampSpeed(y + delta)];
		return i === index ? [x, target] : [x, y];
	});
	return { ...edit, points, curveChanged: true };
}

export function setTime(edit: RammingEdit, seconds: number): RammingEdit {
	const time = clampTime(seconds);
	if (time === edit.time) return edit;
	return { ...edit, time, curveChanged: true };
}

/** Chaîne résultante : exacte si la courbe n'a pas changé. */
export function finishEdit(edit: RammingEdit): string {
	const r = parseRamming(edit.original);
	const lw = clampPercent(edit.lineWidth);
	const sp = clampPercent(edit.spacing);
	if (!edit.curveChanged) {
		if (lw === r.lineWidth && sp === r.spacing) return edit.original;
		return replaceLineParams(edit.original, lw, sp);
	}
	const speeds = sampleSpeeds(edit.points, edit.time) ?? r.speeds;
	return serializeRamming({ lineWidth: lw, spacing: sp, speeds, points: edit.points });
}

/** Points visibles (dans la durée) — les autres sont conservés mais ignorés. */
export function visiblePoints(edit: Pick<RammingEdit, 'points' | 'time'>): number[] {
	const out: number[] = [];
	edit.points.forEach(([x], i) => {
		if (x >= 0 && x <= edit.time) out.push(i);
	});
	return out;
}
