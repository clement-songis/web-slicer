// Multi-filament au niveau du projet (parité Orca : couleurs des filaments et
// volumes de purge appartiennent à la configuration du projet, pas au preset
// filament — Preset.cpp exclut `filament_colour` des options filament et
// `flush_volumes_matrix` vit dans la config projet). Stockés dans la scène
// (`scene.projectConfig`, clés Orca), appliqués par le backend au tranchage
// (contrat : phase15/BACKEND.md). Logique pure, testée.

/** Surcharges projet multi-filament (clés Orca). */
export interface ProjectFilamentConfig {
	/** Couleur par slot (`#RRGGBB`), `''` = couleur du preset. */
	filament_colour?: string[];
	/** Matrice NxN aplatie (mm³), ligne = filament de départ. */
	flush_volumes_matrix?: number[];
	flush_multiplier?: number;
	/** Position de la tour d'essuyage par plateau (coin min, mm) — Orca
	 *  wipe_tower_x / wipe_tower_y (coFloats, config projet). */
	wipe_tower_x?: number[];
	wipe_tower_y?: number[];
}

/** Volume de purge par défaut d'Orca (PrintConfig : 280 mm³ hors diagonale). */
export const DEFAULT_FLUSH = 280;
/** Multiplicateur par défaut d'Orca (PrintConfig flush_multiplier = {0.3}). */
export const DEFAULT_FLUSH_MULTIPLIER = 0.3;

const HEX = /^#[0-9a-f]{6}$/i;

/** Normalise une config lue d'un document (valeurs invalides écartées). */
export function parseProjectConfig(raw: unknown): ProjectFilamentConfig {
	if (!raw || typeof raw !== 'object') return {};
	const o = raw as Record<string, unknown>;
	const out: ProjectFilamentConfig = {};
	if (Array.isArray(o.filament_colour))
		out.filament_colour = o.filament_colour.map((c) =>
			typeof c === 'string' && HEX.test(c) ? c.toLowerCase() : ''
		);
	if (Array.isArray(o.flush_volumes_matrix)) {
		const m = o.flush_volumes_matrix.map(Number);
		if (m.every(Number.isFinite) && Number.isInteger(Math.sqrt(m.length)))
			out.flush_volumes_matrix = m;
	}
	for (const key of ['wipe_tower_x', 'wipe_tower_y'] as const) {
		const v = o[key];
		if (Array.isArray(v) && v.length && v.every((n) => Number.isFinite(Number(n))))
			out[key] = v.map(Number);
	}
	const k = Number(o.flush_multiplier);
	if (o.flush_multiplier !== undefined && Number.isFinite(k) && k > 0) out.flush_multiplier = k;
	return out;
}

/** Config sans entrées vides (rien à persister → `null`). */
export function compactProjectConfig(cfg: ProjectFilamentConfig): ProjectFilamentConfig | null {
	const out: ProjectFilamentConfig = {};
	if (cfg.filament_colour?.some((c) => c)) {
		const colours = [...cfg.filament_colour];
		while (colours.length && !colours[colours.length - 1]) colours.pop();
		out.filament_colour = colours;
	}
	if (cfg.flush_volumes_matrix?.length) out.flush_volumes_matrix = [...cfg.flush_volumes_matrix];
	// Choix explicite conservé tel quel (même 1 : le défaut effectif vient du
	// preset, 0,3 chez Orca, et ne doit pas être réappliqué en silence).
	if (cfg.flush_multiplier !== undefined) out.flush_multiplier = cfg.flush_multiplier;
	if (cfg.wipe_tower_x?.length && cfg.wipe_tower_y?.length) {
		out.wipe_tower_x = [...cfg.wipe_tower_x];
		out.wipe_tower_y = [...cfg.wipe_tower_y];
	}
	return Object.keys(out).length ? out : null;
}

/** Couleurs effectives : surcharge projet du slot, sinon couleur du preset. */
export function effectiveColours(
	presetColours: readonly string[],
	cfg: ProjectFilamentConfig
): string[] {
	return presetColours.map((c, i) => cfg.filament_colour?.[i] || c);
}

/** Pose (ou efface avec `null`) la couleur projet d'un slot. */
export function setSlotColour(
	cfg: ProjectFilamentConfig,
	slot: number,
	colour: string | null
): ProjectFilamentConfig {
	const colours = [...(cfg.filament_colour ?? [])];
	while (colours.length <= slot) colours.push('');
	colours[slot] = colour && HEX.test(colour) ? colour.toLowerCase() : '';
	return { ...cfg, filament_colour: colours };
}

/** Matrice carrée NxN : conserve le coin commun, complète à `DEFAULT_FLUSH`
 *  (0 sur la diagonale). */
export function resizeFlushMatrix(flat: readonly number[] | undefined, n: number): number[] {
	const old = flat?.length ? Math.round(Math.sqrt(flat.length)) : 0;
	const out: number[] = [];
	for (let r = 0; r < n; r++)
		for (let c = 0; c < n; c++)
			out.push(r === c ? 0 : r < old && c < old ? flat![r * old + c] : DEFAULT_FLUSH);
	return out;
}

/** Ajout d'un slot : matrice agrandie (le reste est inchangé). */
export function afterSlotAdded(cfg: ProjectFilamentConfig, count: number): ProjectFilamentConfig {
	if (!cfg.flush_volumes_matrix) return cfg;
	return { ...cfg, flush_volumes_matrix: resizeFlushMatrix(cfg.flush_volumes_matrix, count) };
}

/** Retrait du slot `slot` : sa couleur et sa ligne/colonne de purge disparaissent,
 *  les slots suivants remontent (comme les extrudeurs des objets). */
export function afterSlotRemoved(cfg: ProjectFilamentConfig, slot: number): ProjectFilamentConfig {
	const out: ProjectFilamentConfig = { ...cfg };
	if (cfg.filament_colour) out.filament_colour = cfg.filament_colour.filter((_, i) => i !== slot);
	const m = cfg.flush_volumes_matrix;
	if (m?.length) {
		const n = Math.round(Math.sqrt(m.length));
		const next: number[] = [];
		for (let r = 0; r < n; r++)
			for (let c = 0; c < n; c++) if (r !== slot && c !== slot) next.push(m[r * n + c]);
		// Un seul filament restant : plus de changement, plus de matrice.
		if (next.length > 1) out.flush_volumes_matrix = next;
		else delete out.flush_volumes_matrix;
	}
	return out;
}

/** Position de la tour du plateau `plate` (valeur du projet, sinon défaut
 *  — Orca get_at : dernière valeur si la liste est plus courte). */
export function towerPosition(
	cfg: ProjectFilamentConfig,
	plate: number,
	fallback: [number, number]
): [number, number] {
	const at = (v: number[] | undefined, d: number) =>
		v?.length ? (v[Math.min(plate, v.length - 1)] ?? d) : d;
	return [at(cfg.wipe_tower_x, fallback[0]), at(cfg.wipe_tower_y, fallback[1])];
}

/** Pose la position de la tour du plateau `plate` (listes complétées). */
export function setTowerPosition(
	cfg: ProjectFilamentConfig,
	plate: number,
	x: number,
	y: number,
	fallback: [number, number]
): ProjectFilamentConfig {
	const xs = [...(cfg.wipe_tower_x ?? [])];
	const ys = [...(cfg.wipe_tower_y ?? [])];
	while (xs.length <= plate) xs.push(xs[xs.length - 1] ?? fallback[0]);
	while (ys.length <= plate) ys.push(ys[ys.length - 1] ?? fallback[1]);
	xs[plate] = x;
	ys[plate] = y;
	return { ...cfg, wipe_tower_x: xs, wipe_tower_y: ys };
}

/** Somme des volumes de purge (mm³) d'une matrice, multiplicateur compris :
 *  ordre de grandeur affiché à côté de la matrice. */
export function maxFlush(cfg: ProjectFilamentConfig, n: number): number {
	const m = resizeFlushMatrix(cfg.flush_volumes_matrix, n);
	return Math.max(0, ...m) * (cfg.flush_multiplier ?? 1);
}
