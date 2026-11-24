// Phase 46 (audit 44 CONFL-a) — conflit de trajets après tranchage :
// Orca ConflictChecker → GLCanvas3D::_set_warning_notification(GCodeConflict),
// niveau SLICING_SERIOUS_WARNING (GLCanvas3D.cpp:10078-10090) :
// « Conflicts of G-code paths have been found at layer %d, Z = %.2lfmm. Please
// separate the conflicted objects farther (%s <-> %s). »
// Le serveur (lot 46) remonte le résultat ; formes acceptées : objet
// { layer, z|height, object1|obj1, object2|obj2 } (noms ou ids).
import { tr } from '#lib/i18n';

export interface GcodeConflict {
	layer: number;
	z: number;
	object1: string;
	object2: string;
}

/** Lit un résultat de conflit (null si absent ou incomplet). */
export function parseConflict(raw: unknown): GcodeConflict | null {
	if (!raw || typeof raw !== 'object') return null;
	const r = raw as Record<string, unknown>;
	const num = (v: unknown) => (typeof v === 'number' ? v : typeof v === 'string' ? Number(v) : NaN);
	const str = (...vs: unknown[]) => vs.find((v): v is string => typeof v === 'string' && v !== '');
	const layer = num(r.layer);
	const z = num(r.z ?? r.height);
	const object1 = str(r.object1, r.obj1, r.object_1, r._obj1);
	const object2 = str(r.object2, r.obj2, r.object_2, r._obj2);
	if (!Number.isFinite(layer) || !Number.isFinite(z) || !object1 || !object2) return null;
	return { layer, z, object1, object2 };
}

/** Texte d'Orca (traduit), Z à 2 décimales. */
export function conflictMessage(c: GcodeConflict): string {
	return tr(
		'Conflicts of G-code paths have been found at layer {layer}, Z = {z}mm. Please separate the conflicted objects farther ({a} <-> {b}).',
		{ layer: c.layer, z: c.z.toFixed(2), a: c.object1, b: c.object2 }
	);
}
