// Réglages caméra/souris d'Orca (Preferences.cpp « Camera », GLCanvas3D.cpp)
// traduits en paramètres des contrôles three.js. Fonctions pures, testées.
import { MOUSE } from 'three';
import type { CameraPreferences, DragAction } from '#lib/settings/preferences';

/** Valeur three.js d'un bouton : rotation, panoramique, ou rien (null). */
export function mouseAction(action: DragAction): MOUSE | null {
	return action === 'rotate' ? MOUSE.ROTATE : action === 'pan' ? MOUSE.PAN : null;
}

export interface ControlsConfig {
	mouseButtons: { LEFT: MOUSE | null; MIDDLE: MOUSE | null; RIGHT: MOUSE | null };
	/** OrbitControls : zoom vers le pointeur (Orca `zoom_to_mouse`). */
	zoomToCursor: boolean;
	/** Signe = sens de la molette (Orca `reverse_mouse_wheel_zoom` : facteur −1). */
	zoomSpeed: number;
	/** Orca `camera_orbit_mult`. */
	rotateSpeed: number;
	/** Navigation pavé tactile : Alt+déplacement = rotation, Maj+déplacement = panoramique. */
	touchpad: boolean;
}

/**
 * Orbite contrainte (défaut Orca) : OrbitControls, Z vertical. Pavé tactile
 * (Orca `camera_navigation_style` = 1) : aucun bouton ne déplace la caméra
 * (GLCanvas3D::is_camera_rotate/pan ne regardent que Alt/Maj sans bouton).
 */
export function orbitConfig(cam: CameraPreferences): ControlsConfig {
	const touchpad = cam.style === 'touchpad';
	return {
		mouseButtons: touchpad
			? { LEFT: null, MIDDLE: null, RIGHT: null }
			: {
					LEFT: mouseAction(cam.leftDrag),
					MIDDLE: mouseAction(cam.middleDrag),
					RIGHT: mouseAction(cam.rightDrag)
				},
		zoomToCursor: cam.zoomToMouse,
		zoomSpeed: cam.reverseWheel ? -1 : 1,
		rotateSpeed: cam.orbitSpeed,
		touchpad
	};
}

/**
 * Caméra libre (Orca `use_free_camera` : boule virtuelle, comme la souris
 * 3Dconnexion) : TrackballControls. Ses défauts three (rotation 1, zoom 1,2,
 * panoramique 0,3) sont gardés, multipliés par le facteur d'orbite.
 */
export function trackballConfig(cam: CameraPreferences): {
	mouseButtons: ControlsConfig['mouseButtons'];
	rotateSpeed: number;
	zoomSpeed: number;
	panSpeed: number;
} {
	const o = orbitConfig(cam);
	return {
		mouseButtons: o.mouseButtons,
		rotateSpeed: 2 * cam.orbitSpeed,
		zoomSpeed: 1.2 * (cam.reverseWheel ? -1 : 1),
		panSpeed: 0.3
	};
}

/**
 * Pavé tactile : déplacement sans bouton → mouvement caméra (pixels).
 * Renvoie l'action à appliquer, ou null.
 */
export function touchpadGesture(
	e: { buttons: number; altKey: boolean; shiftKey: boolean },
	enabled: boolean
): 'rotate' | 'pan' | null {
	if (!enabled || e.buttons !== 0) return null;
	if (e.altKey && !e.shiftKey) return 'rotate';
	if (e.shiftKey && !e.altKey) return 'pan';
	return null;
}
