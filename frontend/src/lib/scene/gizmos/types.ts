// Types partagés des gizmos de transformation (T052).

/** Mode du gizmo de transformation (déplacer / tourner / mettre à l'échelle). */
export type GizmoMode = 'translate' | 'rotate' | 'scale';

/** Libellés courts des modes (clés i18n anglaises, traduites au rendu), pour la barre d'outils. */
export const GIZMO_MODE_LABELS: Record<GizmoMode, string> = {
	translate: 'Move',
	rotate: 'Rotate||gizmo',
	scale: 'Scale||gizmo'
};
