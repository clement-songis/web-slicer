import { describe, expect, it } from 'vitest';
import {
	bytesToMb,
	generatePassword,
	invitationLink,
	mbToBytes,
	parseManaged,
	upsertManaged
} from './admin';

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
	it('mémorise les comptes gérés', () => {
		const a = { id: '1', email: 'a@x.fr', role: 'user' };
		const list = upsertManaged(upsertManaged([], a), { id: '2', email: 'b@x.fr', role: 'admin' });
		expect(list.map((u) => u.id)).toEqual(['2', '1']);
		expect(upsertManaged(list, { ...a, role: 'admin' }).map((u) => u.id)).toEqual(['1', '2']);
		expect(parseManaged(JSON.stringify(list))).toHaveLength(2);
		expect(parseManaged('{oops')).toEqual([]);
		expect(parseManaged(JSON.stringify([{ id: 3 }]))).toEqual([]);
	});
});
