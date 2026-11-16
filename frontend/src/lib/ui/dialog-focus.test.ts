import { afterEach, describe, expect, test, vi } from 'vitest';
import { dialogFocus } from './dialog-focus';

// Environnement `node` (pas de DOM) : document et éléments minimaux simulés.
type Fake = {
	focus: ReturnType<typeof vi.fn>;
	isConnected: boolean;
	hasAttribute: () => boolean;
	contains: (e: unknown) => boolean;
};
function el(children: unknown[] = []): Fake {
	const self: Fake = {
		focus: vi.fn(() => {
			(globalThis as { document: { activeElement: unknown } }).document.activeElement = self;
		}),
		isConnected: true,
		hasAttribute: () => false,
		contains: (e) => e === self || children.includes(e)
	};
	return self;
}

afterEach(() => {
	delete (globalThis as { document?: unknown }).document;
});

describe('dialogFocus', () => {
	test('moves focus into the dialog on open and restores it on close', () => {
		const opener = el();
		const dialog = el();
		(globalThis as { document: unknown }).document = { activeElement: opener };
		const action = dialogFocus(dialog as unknown as HTMLElement);
		expect(dialog.focus).toHaveBeenCalledOnce();
		action.destroy();
		expect(opener.focus).toHaveBeenCalledOnce();
	});

	test('keeps focus on a field already focused inside the dialog', () => {
		const input = el();
		const dialog = el([input]);
		(globalThis as { document: unknown }).document = { activeElement: input };
		dialogFocus(dialog as unknown as HTMLElement);
		expect(dialog.focus).not.toHaveBeenCalled();
	});

	test('does not restore focus to a detached opener', () => {
		const opener = el();
		const dialog = el();
		(globalThis as { document: unknown }).document = { activeElement: opener };
		const action = dialogFocus(dialog as unknown as HTMLElement);
		opener.isConnected = false;
		action.destroy();
		expect(opener.focus).not.toHaveBeenCalled();
	});
});
