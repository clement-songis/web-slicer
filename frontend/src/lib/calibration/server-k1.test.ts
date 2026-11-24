import { beforeEach, describe, expect, it, vi } from 'vitest';

// Lot 42 K1 : le serveur nomme le projet de calibration (nom unique) ; le
// client ne renomme plus et ne code plus de libellé provisoire par type.
const calls: string[] = [];
vi.mock('#lib/api/projects', () => ({
	createProject: vi.fn(async (body: { name: string }) => {
		calls.push('create:' + body.name);
		return { id: 'p1', name: body.name };
	}),
	deleteProject: vi.fn(async (id: string) => {
		calls.push('delete:' + id);
	}),
	renameProject: vi.fn(async () => {
		calls.push('rename');
	})
}));
const post = vi.fn();
vi.mock('#lib/api/client', () => ({
	api: { post: (...args: unknown[]) => post(...args) }
}));

import { createServerCalibration, pressureAdvanceRequest } from './server-calib';

describe('createServerCalibration (lot 42 K1)', () => {
	beforeEach(() => {
		calls.length = 0;
		post.mockReset();
	});
	it('reprend le nom unique posé par le serveur, sans renommage client', async () => {
		post.mockResolvedValue({
			name: 'Pressure Advance Test (2)',
			calibration_name: 'Pressure Advance Test',
			version: 2
		});
		const res = await createServerCalibration(
			pressureAdvanceRequest({ start: 0, end: 0.1, step: 0.005 }),
			{}
		);
		expect(res).toEqual({ id: 'p1', name: 'Pressure Advance Test (2)' });
		expect(post).toHaveBeenCalledWith('/projects/p1/calibration', expect.any(Object));
		expect(calls).toEqual(['create:Calibration']);
	});
	it('retire le projet créé si la calibration échoue', async () => {
		post.mockRejectedValue(new Error('boom'));
		await expect(
			createServerCalibration(pressureAdvanceRequest({ start: 0, end: 0.1, step: 0.005 }), {})
		).rejects.toThrow('boom');
		expect(calls).toEqual(['create:Calibration', 'delete:p1']);
	});
});
