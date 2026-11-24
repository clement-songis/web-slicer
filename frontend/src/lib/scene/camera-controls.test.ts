import { describe, expect, it } from 'vitest';
import { MOUSE } from 'three';
import { CAMERA_DEFAULTS } from '#lib/settings/preferences';
import { mouseAction, orbitConfig, touchpadGesture, trackballConfig } from './camera-controls';

describe('contrôles caméra (Orca « Camera »)', () => {
	it('défaut : gauche = rotation, milieu/droit = panoramique, molette normale', () => {
		const c = orbitConfig(CAMERA_DEFAULTS);
		expect(c.mouseButtons).toEqual({ LEFT: MOUSE.ROTATE, MIDDLE: MOUSE.PAN, RIGHT: MOUSE.PAN });
		expect(c.zoomSpeed).toBe(1);
		expect(c.zoomToCursor).toBe(false);
		expect(c.rotateSpeed).toBe(1);
		expect(c.touchpad).toBe(false);
	});
	it('boutons, zoom vers le pointeur, molette inversée, vitesse', () => {
		const c = orbitConfig({
			...CAMERA_DEFAULTS,
			leftDrag: 'pan',
			rightDrag: 'rotate',
			middleDrag: 'none',
			zoomToMouse: true,
			reverseWheel: true,
			orbitSpeed: 0.5
		});
		expect(c.mouseButtons).toEqual({ LEFT: MOUSE.PAN, MIDDLE: null, RIGHT: MOUSE.ROTATE });
		expect(c.zoomToCursor).toBe(true);
		expect(c.zoomSpeed).toBe(-1);
		expect(c.rotateSpeed).toBe(0.5);
		expect(mouseAction('none')).toBeNull();
	});
	it('pavé tactile : aucun bouton, Alt = rotation, Maj = panoramique (sans bouton)', () => {
		const c = orbitConfig({ ...CAMERA_DEFAULTS, style: 'touchpad' });
		expect(c.mouseButtons).toEqual({ LEFT: null, MIDDLE: null, RIGHT: null });
		expect(touchpadGesture({ buttons: 0, altKey: true, shiftKey: false }, true)).toBe('rotate');
		expect(touchpadGesture({ buttons: 0, altKey: false, shiftKey: true }, true)).toBe('pan');
		expect(touchpadGesture({ buttons: 1, altKey: true, shiftKey: false }, true)).toBeNull();
		expect(touchpadGesture({ buttons: 0, altKey: true, shiftKey: true }, true)).toBeNull();
		expect(touchpadGesture({ buttons: 0, altKey: true, shiftKey: false }, false)).toBeNull();
	});
	it('caméra libre : boule virtuelle, molette inversée conservée', () => {
		const t = trackballConfig({
			...CAMERA_DEFAULTS,
			freeCamera: true,
			reverseWheel: true,
			orbitSpeed: 2
		});
		expect(t.rotateSpeed).toBe(4);
		expect(t.zoomSpeed).toBeLessThan(0);
	});
});
