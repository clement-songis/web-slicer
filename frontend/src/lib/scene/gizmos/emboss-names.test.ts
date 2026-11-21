import { describe, expect, it } from 'vitest';
import { svgVolumeName, textVolumeName } from './emboss-names';

describe('noms Texte / SVG (audit 41 V4, Orca create_volume_name / volume_name)', () => {
	it('texte : le texte, retours à la ligne → espaces', () => {
		expect(textVolumeName('Orca 41')).toBe('Orca 41');
		expect(textVolumeName('A\nB')).toBe('A B');
		expect(textVolumeName('   ')).toBe('Text');
	});
	it('SVG : nom de fichier sans extension, sinon « SVG shape »', () => {
		expect(svgVolumeName('logo.svg')).toBe('logo');
		expect(svgVolumeName('dir/my.logo.svg')).toBe('my.logo');
		expect(svgVolumeName(undefined)).toBe('SVG shape');
		expect(svgVolumeName('')).toBe('SVG shape');
	});
});
