// Types partagés des gizmos de transformation (T052).

/** Mode du gizmo de transformation (déplacer / tourner / mettre à l'échelle). */
export type GizmoMode = 'translate' | 'rotate' | 'scale';

/** Libellés courts (FR) des modes, pour la barre d'outils. */
export const GIZMO_MODE_LABELS: Record<GizmoMode, string> = {
	translate: 'Déplacer',
	rotate: 'Tourner',
	scale: 'Échelle'
};
