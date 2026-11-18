import { describe, expect, it } from 'vitest';
import { UI_LAYOUT } from '../../generated/ui-layout';
import { resolveDynamicOptions } from './dynamic-options';

describe('lignes à widget d Orca ajoutées au layout', () => {
	it('« Ramming settings… » en fin de section mono-extrudeur MM (Tab.cpp:4191)', () => {
		const page = resolveDynamicOptions(UI_LAYOUT).find(
			(p) => p.kind === 'filament' && p.title === 'Multimaterial'
		)!;
		const section = page.sections.find(
			(s) => s.title === 'Tool change parameters with single extruder MM printers'
		)!;
		expect(section.options[section.options.length - 1]).toBe('filament_ramming_parameters');
		// Une seule fois, et le layout généré reste intact.
		expect(section.options.filter((o) => o === 'filament_ramming_parameters')).toHaveLength(1);
		const raw = UI_LAYOUT.find((p) => p.kind === 'filament' && p.title === 'Multimaterial')!;
		expect(raw.sections.flatMap((s) => s.options)).not.toContain('filament_ramming_parameters');
	});
});
