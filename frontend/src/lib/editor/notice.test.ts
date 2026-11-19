import { describe, expect, it } from 'vitest';
import { nextNotice } from './notice';

describe('nextNotice', () => {
	it('numérote chaque message, même identique au précédent', () => {
		const a = nextNotice(null, 'Projet enregistré.');
		const b = nextNotice(a, 'Projet enregistré.');
		expect(a).toEqual({ text: 'Projet enregistré.', seq: 1 });
		expect(b).toEqual({ text: 'Projet enregistré.', seq: 2 });
	});
	it('ignore null / vide (la notification précédente reste)', () => {
		const a = nextNotice(null, 'x');
		expect(nextNotice(a, null)).toBe(a);
		expect(nextNotice(a, '')).toBe(a);
		expect(nextNotice(null, undefined)).toBeNull();
	});
});
