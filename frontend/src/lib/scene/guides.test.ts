import { describe, expect, it } from 'vitest';
import { cutGuidePlane, cutOffset, cutWorld, guideRotation, measureGuides } from './guides';

const bounds = {
	min: [-10, -5, -10] as [number, number, number],
	max: [10, 5, 10] as [number, number, number]
};

describe('repères de la vue 3D (phase 26)', () => {
	it('décalage local ↔ hauteur monde (origine + échelle de l’objet)', () => {
		const frame = {
			origin: [128, 128, 10] as [number, number, number],
			scale: [1, 1, 2] as [number, number, number]
		};
		expect(cutWorld(0, 'z', frame)).toBe(10);
		expect(cutWorld(5, 'z', frame)).toBe(20);
		expect(cutOffset(20, 'z', frame)).toBe(5);
		expect(cutOffset(cutWorld(-3, 'x', frame), 'x', frame)).toBe(-3);
	});

	it('plan horizontal (Z) : centré sur l’objet, à la hauteur de coupe, emprise + marge', () => {
		const p = cutGuidePlane('z', 5, bounds, { origin: [100, 50, 10], scale: [1, 1, 1] }, 4);
		expect(p.center).toEqual([100, 50, 15]);
		expect(p.normal).toEqual([0, 0, 1]);
		expect(p.size).toEqual([28, 18]);
		expect(guideRotation(p.normal)).toEqual([0, 0, 0]);
	});

	it('plans verticaux (X, Y) : dimensions dans le repère du plan tourné', () => {
		const px = cutGuidePlane('x', 2, bounds, undefined, 0);
		expect(px.center).toEqual([2, 0, 0]);
		expect(px.size).toEqual([20, 10]); // (Z, Y)
		expect(guideRotation(px.normal)).toEqual([0, Math.PI / 2, 0]);
		const py = cutGuidePlane('y', -1, bounds, undefined, 0);
		expect(py.center).toEqual([0, -1, 0]);
		expect(py.size).toEqual([20, 20]); // (X, Z)
		expect(guideRotation(py.normal)).toEqual([-Math.PI / 2, 0, 0]);
	});

	it('mesure : garde les deux derniers points cliqués', () => {
		expect(measureGuides([]).points).toEqual([]);
		expect(
			measureGuides([
				[0, 0, 0],
				[1, 2, 3],
				[4, 5, 6]
			]).points
		).toEqual([
			[1, 2, 3],
			[4, 5, 6]
		]);
	});
});
