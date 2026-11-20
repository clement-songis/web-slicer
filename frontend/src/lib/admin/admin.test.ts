import { describe, expect, it } from 'vitest';
import {
	bytesToMb,
	generatePassword,
	invitationLink,
	mbToBytes,
	pageLabel,
	parseSliceLimit,
	quotaMbToBytes
} from './admin';

describe('quotas et liste des comptes (lot 32)', () => {
	it('quota de stockage : Mo → octets, 0 / vide = illimité', () => {
		expect(quotaMbToBytes(500)).toBe(524288000);
		expect(quotaMbToBytes(0)).toBe(0);
		expect(quotaMbToBytes(null)).toBe(0);
		expect(quotaMbToBytes(Number.NaN)).toBe(0);
		expect(quotaMbToBytes(-1)).toBeNull();
	});
	it('tranchages simultanés : entier 0..=1000', () => {
		expect(parseSliceLimit(2)).toBe(2);
		expect(parseSliceLimit(0)).toBe(0);
		expect(parseSliceLimit(null)).toBe(0);
		expect(parseSliceLimit(1.5)).toBeNull();
		expect(parseSliceLimit(1001)).toBeNull();
		expect(parseSliceLimit(-1)).toBeNull();
	});
	it('libellé de pagination', () => {
		expect(pageLabel(1, 25, 0)).toBe('Aucun compte');
		expect(pageLabel(1, 25, 7)).toBe('1–7 sur 7');
		expect(pageLabel(3, 50, 132)).toBe('101–132 sur 132');
	});
});

describe('administration', () => {
	it('convertit la limite d’upload', () => {
		expect(bytesToMb(524288000)).toBe(500);
		expect(bytesToMb(BigInt(1572864))).toBe(1.5);
		expect(mbToBytes(500)).toBe(524288000);
		expect(mbToBytes(0)).toBeNull();
		expect(mbToBytes(Number.NaN)).toBeNull();
	});
	it('construit le lien d’invitation', () => {
		expect(invitationLink('https://slicer.example/', 'ab c')).toBe(
			'https://slicer.example/register?invite=ab%20c'
		);
	});
	it('génère un mot de passe de 14 caractères sans ambiguïté', () => {
		const pw = generatePassword((n) => new Uint8Array(n).map((_, i) => i * 7));
		expect(pw).toHaveLength(14);
		expect(pw).not.toMatch(/[0O1lI]/);
		expect(generatePassword()).toHaveLength(14);
	});
});
