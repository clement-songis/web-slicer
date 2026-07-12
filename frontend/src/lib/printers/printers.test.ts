// Tests de la logique pure du suivi d'imprimantes (T077) : réducteur d'état,
// métadonnées, disponibilité des contrôles, formatage.
import { describe, expect, test } from 'vitest';
import type { PrinterStatusResponse, ServerEvent } from '$lib/api/types';
import {
	applyPrinterStatus,
	canCancel,
	canPause,
	canResume,
	formatTemp,
	fromStatusResponse,
	progressPercent,
	stateMeta,
	type StatusMap
} from './printers';

const statusEvent = (over: Partial<Extract<ServerEvent, { event: 'printer.status' }>> = {}) =>
	({
		event: 'printer.status',
		printer_id: 'p1',
		state: 'printing',
		filename: 'benchy.gcode',
		progress: 0.25,
		extruder_temp: 210,
		extruder_target: 220,
		bed_temp: 60,
		bed_target: 60,
		...over
	}) satisfies ServerEvent;

describe('applyPrinterStatus', () => {
	test('indexes the latest status by printer id', () => {
		const map = applyPrinterStatus({}, statusEvent());
		expect(map.p1.state).toBe('printing');
		expect(map.p1.progress).toBe(0.25);
		expect(map.p1.extruderTemp).toBe(210);
		expect(map.p1.bedTarget).toBe(60);
	});

	test('overwrites only the affected printer, immutably', () => {
		const base: StatusMap = applyPrinterStatus({}, statusEvent({ printer_id: 'p2' }));
		const next = applyPrinterStatus(base, statusEvent({ printer_id: 'p1', progress: 0.9 }));
		expect(next).not.toBe(base);
		expect(next.p2).toBe(base.p2); // p2 inchangé
		expect(next.p1.progress).toBe(0.9);
	});

	test('ignores non-status events', () => {
		const map = applyPrinterStatus({}, statusEvent());
		const event: ServerEvent = { event: 'model.converted', model_id: 'm', mesh_url: '/x' };
		expect(applyPrinterStatus(map, event)).toBe(map);
	});
});

describe('fromStatusResponse', () => {
	test('maps the REST snapshot to a view', () => {
		const resp: PrinterStatusResponse = {
			state: 'paused',
			filename: 'a.gcode',
			progress: 0.5,
			print_duration: 120,
			extruder_temp: 200,
			extruder_target: 0,
			bed_temp: 55,
			bed_target: 60
		};
		const view = fromStatusResponse(resp);
		expect(view.state).toBe('paused');
		expect(view.extruderTarget).toBe(0);
		expect(view.bedTarget).toBe(60);
	});
});

describe('stateMeta', () => {
	test('known states get labels', () => {
		expect(stateMeta('printing').label).toBe('Impression');
		expect(stateMeta('paused').label).toBe('En pause');
		expect(stateMeta('error').label).toBe('Erreur');
	});
	test('unknown state falls back neutrally', () => {
		expect(stateMeta('weird').label).toBe('weird');
		expect(stateMeta('').label).toBe('Inconnu');
	});
});

describe('control availability', () => {
	test('pause only while printing', () => {
		expect(canPause('printing')).toBe(true);
		expect(canPause('paused')).toBe(false);
	});
	test('resume only while paused', () => {
		expect(canResume('paused')).toBe(true);
		expect(canResume('printing')).toBe(false);
	});
	test('cancel while printing or paused', () => {
		expect(canCancel('printing')).toBe(true);
		expect(canCancel('paused')).toBe(true);
		expect(canCancel('complete')).toBe(false);
	});
});

describe('formatting', () => {
	test('progressPercent clamps to 0–100', () => {
		expect(progressPercent(0.25)).toBe(25);
		expect(progressPercent(-1)).toBe(0);
		expect(progressPercent(2)).toBe(100);
	});
	test('formatTemp shows target only when set', () => {
		expect(formatTemp(210.4, 220)).toBe('210 °C → 220 °C');
		expect(formatTemp(24.6, 0)).toBe('25 °C');
	});
});
