import { describe, expect, test } from 'vitest';
import { EnginePaintClient } from './engine-paint';

/** Faux WebSocket : répond comme le backend (open → opened, stroke → facets). */
class FakeSocket {
	onopen: (() => void) | null = null;
	onerror: (() => void) | null = null;
	onclose: (() => void) | null = null;
	onmessage: ((ev: MessageEvent) => void) | null = null;
	sent: Record<string, unknown>[] = [];
	constructor() {
		setTimeout(() => this.onopen?.(), 0);
	}
	send(data: string) {
		const msg = JSON.parse(data);
		this.sent.push(msg);
		const reply =
			msg.type === 'open'
				? { type: 'opened', seq: msg.seq, triangles: 12 }
				: msg.type === 'stroke'
					? { type: 'facets', seq: msg.seq, facets: { '2': '4433441' } }
					: { type: 'error', seq: msg.seq, message: 'non' };
		setTimeout(() => this.onmessage?.({ data: JSON.stringify(reply) } as MessageEvent), 0);
	}
	close() {
		this.onclose?.();
	}
}

describe('EnginePaintClient', () => {
	test('ouvre la session une fois puis renvoie les facettes du moteur', async () => {
		let socket!: FakeSocket;
		const client = new EnginePaintClient('ws://x', () => {
			socket = new FakeSocket();
			return socket as unknown as WebSocket;
		});
		const stroke = {
			points: [[1, 2, 3]] as [number, number, number][],
			facets: [2],
			camera_pos: [0, 0, 100] as [number, number, number],
			radius: 2,
			cursor: 'circle' as const,
			state: 1,
			trafo: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
		};
		expect(await client.stroke('o', 'm', 'supports', {}, stroke)).toEqual({ '2': '4433441' });
		await client.stroke('o', 'm', 'supports', { '2': '4433441' }, stroke);
		expect(socket.sent.map((m) => m.type)).toEqual(['open', 'stroke', 'stroke']);
		expect(socket.sent[2].facets).toEqual({ '2': '4433441' });
		await expect(
			client.layerAdjust({
				model_id: 'm',
				values: {},
				profile: [],
				z: 1,
				delta: 0.005,
				band_width: 2,
				action: 'increase'
			})
		).rejects.toThrow('non');
	});
});
