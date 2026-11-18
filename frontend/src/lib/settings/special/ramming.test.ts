import { describe, expect, it } from 'vitest';
import {
	DEFAULT_RAMMING,
	clampTime,
	finishEdit,
	formatStreamNumber,
	movePoint,
	parseRamming,
	rammingTime,
	rammingVolume,
	sampleSpeeds,
	serializeRamming,
	setTime,
	startEdit,
	visiblePoints
} from './ramming';

describe('ramming (filament_ramming_parameters)', () => {
	it('lit la chaîne par défaut d Orca : largeur, espacement, 10 débits, 11 points', () => {
		const r = parseRamming(DEFAULT_RAMMING);
		expect(r.lineWidth).toBe(120);
		expect(r.spacing).toBe(100);
		expect(r.speeds).toHaveLength(10);
		expect(rammingTime(r)).toBe(2.5);
		expect(r.points).toHaveLength(11);
		expect(r.points[0]).toEqual([0.05, 6.6]);
		expect(r.points[10]).toEqual([4.95, 7.6]);
	});

	it('chaîne sans points (ancien format) : points posés sur les débits', () => {
		const r = parseRamming('120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0|');
		expect(r.speeds).toHaveLength(10);
		expect(r.points[0]).toEqual([0.05, 6.6]);
		expect(r.points[1]).toEqual([0.45, 6.8]);
		expect(r.points[10][1]).toBe(10);
	});

	it('écrit les nombres comme un flux C++ (6 chiffres significatifs)', () => {
		expect(formatStreamNumber(6.6)).toBe('6.6');
		expect(formatStreamNumber(10)).toBe('10');
		expect(formatStreamNumber(7.59999)).toBe('7.59999');
		expect(formatStreamNumber(1 / 3)).toBe('0.333333');
		expect(formatStreamNumber(123.456789)).toBe('123.457');
		expect(formatStreamNumber(0)).toBe('0');
		expect(formatStreamNumber(0.0000123)).toBe('1.23e-05');
	});

	it('conserve la chaîne exacte sans modification', () => {
		for (const raw of [
			DEFAULT_RAMMING,
			'120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0|',
			'90  110 5 5 5 5| 0.1 5 0.9 5'
		]) {
			expect(finishEdit(startEdit(raw))).toBe(raw);
		}
	});

	it('largeur / espacement seuls : le reste de la chaîne est intact', () => {
		const e = { ...startEdit(DEFAULT_RAMMING), lineWidth: 140, spacing: 85 };
		expect(finishEdit(e)).toBe(DEFAULT_RAMMING.replace(/^120 100/, '140 85'));
		// Bornes du SpinInput d'Orca (10–300 %).
		expect(finishEdit({ ...startEdit(DEFAULT_RAMMING), lineWidth: 900, spacing: 1 })).toMatch(
			/^300 10 /
		);
	});

	it('courbe plate : débits constants, volume = débit × durée', () => {
		const pts: [number, number][] = [
			[0.05, 12],
			[0.95, 12],
			[1.95, 12]
		];
		const speeds = sampleSpeeds(pts, 2)!;
		expect(speeds).toHaveLength(8);
		for (const s of speeds) expect(s).toBeCloseTo(12, 0);
		expect(rammingVolume(pts, 2)).toBeGreaterThan(22);
		expect(rammingVolume(pts, 2)).toBeLessThan(26);
	});

	it('spline : retrouve les débits par défaut d Orca à ± 0,3 mm³/s', () => {
		const r = parseRamming(DEFAULT_RAMMING);
		const speeds = sampleSpeeds(r.points, rammingTime(r))!;
		expect(speeds).toHaveLength(r.speeds.length);
		speeds.forEach((s, i) => expect(Math.abs(s - r.speeds[i])).toBeLessThan(0.3));
	});

	it('déplacer un point recalcule les débits et réécrit la chaîne au format Orca', () => {
		let e = startEdit(DEFAULT_RAMMING);
		e = movePoint(e, 2, 20);
		const out = finishEdit(e);
		const r = parseRamming(out);
		expect(out).toMatch(/^120 100 [\d. ]+\| 0\.05 6\.6 0\.45 6\.8 0\.95 20 /);
		expect(r.speeds).toHaveLength(10);
		expect(Math.max(...r.speeds)).toBeGreaterThan(15);
		expect(serializeRamming(r)).toBe(out);
	});

	it('débit constant : tous les points suivent le même décalage (bornés à 0–60)', () => {
		const e = movePoint(startEdit(DEFAULT_RAMMING), 0, 8.6, true);
		expect(e.points[0][1]).toBeCloseTo(8.6);
		expect(e.points[5][1]).toBeCloseTo(12);
		const top = movePoint(startEdit(DEFAULT_RAMMING), 0, 100, true);
		expect(Math.max(...top.points.map((p) => p[1]))).toBe(60);
	});

	it('durée : pas de 0,25 s, 5 s max ; nombre de débits suivi', () => {
		expect(clampTime(1.1)).toBe(1);
		expect(clampTime(9)).toBe(5);
		const e = setTime(startEdit(DEFAULT_RAMMING), 4);
		expect(parseRamming(finishEdit(e)).speeds).toHaveLength(16);
		expect(visiblePoints(e)).toHaveLength(9);
		const zero = setTime(startEdit(DEFAULT_RAMMING), 0);
		expect(parseRamming(finishEdit(zero)).speeds).toHaveLength(0);
	});
});
