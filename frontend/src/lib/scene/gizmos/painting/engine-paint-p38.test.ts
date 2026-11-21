import { describe, expect, test } from 'vitest';
import { EnginePaintClient } from './engine-paint';

/** Faux WebSocket : répond comme le backend du lot 38 (gap_fill, remap, preview). */
class FakeSocket {
	onopen: (() => void) | null = null;
	onerror: (() => void) | null = null;
	onclose: (() => void) | null = null;
	onmessage: ((ev: MessageEvent) => void) | null = null;
	sent: Record<string, unknown>[] = [];
	delays: Record<string, number> = { preview: 0 };
	constructor() {
		setTimeout(() => this.onopen?.(), 0);
	}
	send(data: string) {
		const msg = JSON.parse(data);
		this.sent.push(msg);
		let reply: Record<string, unknown>;
		if (msg.type === 'open') reply = { type: 'opened', seq: msg.seq, triangles: 12 };
		else if (msg.type === 'preview')
			reply = {
				type: 'preview',
				seq: msg.seq,
				object_id: msg.object_id,
				channel: msg.channel,
				vertices: [0, 0, 0, 1, 0, 0, 0, 1, 0],
				truncated: false
			};
		else if (msg.type === 'gap_fill' && msg.area > 5)
			reply = { type: 'error', seq: msg.seq, message: 'aire hors bornes' };
		else reply = { type: 'facets', seq: msg.seq, facets: { '1': '8' } };
		setTimeout(() => this.onmessage?.({ data: JSON.stringify(reply) } as MessageEvent), 0);
	}
	close() {
		this.onclose?.();
	}
}

describe('EnginePaintClient — messages du lot 38', () => {
	test('gap_fill, remap et preview (avec aperçus dépassés signalés)', async () => {
		let socket!: FakeSocket;
		const client = new EnginePaintClient('ws://x', () => {
			socket = new FakeSocket();
			return socket as unknown as WebSocket;
		});
		expect(await client.gapFill('o', 'm', 'mmu', { '1': '4' }, 2.4)).toEqual({ '1': '8' });
		expect(socket.sent[1]).toMatchObject({ type: 'gap_fill', channel: 'mmu', area: 2.4 });
		await expect(client.gapFill('o', 'm', 'mmu', {}, 6)).rejects.toThrow('aire hors bornes');

		expect(await client.remap('o', 'm', { '1': '4' }, [2, 1])).toEqual({ '1': '8' });
		expect(socket.sent.at(-1)).toMatchObject({ type: 'remap', channel: 'mmu', map: [2, 1] });

		const stroke = {
			points: [[1, 2, 3]] as [number, number, number][],
			facets: [2],
			camera_pos: [0, 0, 100] as [number, number, number],
			radius: 2,
			cursor: 'pointer' as const,
			tool: 'bucket_fill' as const,
			fill_angle: -1,
			state: 2,
			trafo: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
		};
		const first = client.preview('o', 'm', 'mmu', {}, stroke);
		const second = client.preview('o', 'm', 'mmu', {}, stroke);
		expect((await first).stale).toBe(true);
		const last = await second;
		expect(last.stale).toBe(false);
		expect(last.vertices).toHaveLength(9);
		expect(socket.sent.at(-1)).toMatchObject({
			type: 'preview',
			stroke: { cursor: 'pointer', tool: 'bucket_fill', fill_angle: -1, splitting: true }
		});
		// Un seul « open » pour l'objet.
		expect(socket.sent.filter((m) => m.type === 'open')).toHaveLength(1);
	});
});
