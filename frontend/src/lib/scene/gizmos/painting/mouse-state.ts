// Audit 42 S2 — boutons de la souris dans les gizmos de peinture d'Orca 2.4.1
// (GLGizmoPainterBase::gizmo_event, GLGizmoPainterBase.cpp:726-748) :
//   - Maj enfoncée : efface (EnforcerBlockerType::NONE), quel que soit le bouton ;
//   - bouton gauche : get_left_button_state_type() — ici l'état choisi dans la
//     barre (Forcer / Bloquer, ou le filament MMU) ;
//   - bouton droit : get_right_button_state_type() — BLOCKER pour les supports
//     et la couture (valeur par défaut de la classe de base), NONE (gomme) pour
//     la surface floue (GLGizmoFuzzySkin.hpp:30), -1 = non géré pour le
//     multi-matériaux (GLGizmoMmuSegmentation.hpp:94 : pas de peinture).
import { BLOCKER } from './facets';
import type { PaintChannel } from './painting';

/** État de peinture « effacer » (EnforcerBlockerType::NONE). */
export const ERASE = 0;

/**
 * État peint par un appui (`button` : 0 gauche, 2 droit), ou `null` si l'appui
 * ne peint pas (bouton du milieu, droit en multi-matériaux).
 */
export function mouseStrokeState(
	channel: PaintChannel,
	button: number,
	shift: boolean,
	selected: number
): number | null {
	if (button !== 0 && button !== 2) return null;
	if (button === 2 && channel === 'mmu') return null;
	if (shift) return ERASE;
	if (button === 0) return selected;
	return channel === 'fuzzy' ? ERASE : BLOCKER;
}
