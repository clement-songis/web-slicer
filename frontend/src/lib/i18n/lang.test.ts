import { describe, expect, it } from 'vitest';
import { syncDocumentLang } from './index';

describe('<html lang> suit la langue (audit 42 A3)', () => {
	it('met à jour documentElement.lang', () => {
		const doc = { documentElement: { lang: 'en' } };
		syncDocumentLang('fr', doc);
		expect(doc.documentElement.lang).toBe('fr');
		syncDocumentLang('en', doc);
		expect(doc.documentElement.lang).toBe('en');
	});
	it('sans document (SSR) : sans effet', () => {
		expect(() => syncDocumentLang('fr', undefined)).not.toThrow();
	});
});
