import { describe, expect, it } from 'bun:test';
import { APP_NAME, appIdentity } from './version';

describe('appIdentity', () => {
	it('compose nom et version', () => {
		expect(appIdentity('0.1.0')).toBe(`${APP_NAME}/0.1.0`);
	});
});
