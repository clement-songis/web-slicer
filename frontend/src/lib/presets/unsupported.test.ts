import { describe, expect, it } from 'vitest';
import { subtractCompatible } from './unsupported';
import type { PresetSummary } from '#lib/api/types';

const p = (id: string, instantiation = true) =>
	({ id, name: id, kind: 'filament', origin: 'system', instantiation }) as unknown as PresetSummary;

describe('presets incompatibles (Orca show_unsupported_presets)', () => {
	it('liste complète moins les compatibles, instanciables seulement', () => {
		expect(subtractCompatible([p('a'), p('b'), p('c', false)], [p('a')]).map((x) => x.id)).toEqual([
			'b'
		]);
	});
});
