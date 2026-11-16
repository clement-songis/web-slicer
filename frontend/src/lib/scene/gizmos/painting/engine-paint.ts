// Canal moteur des interactions continues (`GET /api/projects/{id}/paint`,
// WebSocket) : peinture par le `TriangleSelector` d'Orca et pinceau de hauteur
// de couche (`adjust_layer_height_profile`), calculés par un worker persistant.
// Le TS ne fait que l'aperçu instantané ; la réponse du moteur remplace l'état
// et c'est elle qui est persistée. Requêtes corrélées par `seq`.
import type { PaintChannel } from './painting';

type Vec3 = [number, number, number];

/** Trait de pinceau (repère du maillage recentré de l'objet). */
export interface EngineStroke {
	points: Vec3[];
	facets: number[];
	camera_pos: Vec3;
	radius: number;
	cursor: 'circle' | 'sphere';
	state: number;
	/** Matrice monde colonne-major (16). */
	trafo: number[];
	splitting?: boolean;
	highlight_deg?: number;
	/** Pinceau (défaut), smart fill ou pot de peinture (`ToolType` d'Orca). */
	tool?: 'brush' | 'smart_fill' | 'bucket_fill';
	/** Angle d'arrêt des remplissages (°, défaut Orca 30). */
	fill_angle?: number;
}

export type LayerEditAction = 'increase' | 'decrease' | 'reduce' | 'smooth';

interface Pending {
	resolve: (msg: Record<string, unknown>) => void;
	reject: (err: Error) => void;
}

/** Fabrique de socket injectable (tests). */
export type SocketFactory = (url: string) => WebSocket;

export function paintWsUrl(projectId: string): string {
	const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${proto}//${location.host}/api/projects/${projectId}/paint`;
}

export class EnginePaintClient {
	private socket: WebSocket | null = null;
	private ready: Promise<WebSocket> | null = null;
	private seq = 0;
	private readonly pending = new Map<number, Pending>();
	private readonly opened = new Map<string, string>(); // object_id → model_id

	constructor(
		private readonly url: string,
		private readonly factory: SocketFactory = (u) => new WebSocket(u)
	) {}

	private connect(): Promise<WebSocket> {
		if (this.ready) return this.ready;
		this.ready = new Promise((resolve, reject) => {
			const socket = this.factory(this.url);
			socket.onopen = () => resolve(socket);
			socket.onerror = () => reject(new Error('canal moteur indisponible'));
			socket.onclose = () => {
				this.socket = null;
				this.ready = null;
				this.opened.clear();
				for (const p of this.pending.values()) p.reject(new Error('canal moteur fermé'));
				this.pending.clear();
			};
			socket.onmessage = (ev: MessageEvent) => {
				let msg: Record<string, unknown>;
				try {
					msg = JSON.parse(String(ev.data));
				} catch {
					return;
				}
				const p = this.pending.get(Number(msg.seq));
				if (!p) return;
				this.pending.delete(Number(msg.seq));
				if (msg.type === 'error') p.reject(new Error(String(msg.message ?? 'erreur moteur')));
				else p.resolve(msg);
			};
			this.socket = socket;
		});
		return this.ready;
	}

	private async request(body: Record<string, unknown>): Promise<Record<string, unknown>> {
		const socket = await this.connect();
		const seq = ++this.seq;
		return new Promise((resolve, reject) => {
			this.pending.set(seq, { resolve, reject });
			socket.send(JSON.stringify({ ...body, seq }));
		});
	}

	/** Ouvre (une fois) la session de peinture d'un objet sur son modèle serveur. */
	async open(objectId: string, modelId: string): Promise<void> {
		if (this.opened.get(objectId) === modelId) return;
		await this.request({ type: 'open', object_id: objectId, model_id: modelId });
		this.opened.set(objectId, modelId);
	}

	/**
	 * Trait moteur : `facets` = état persisté courant du canal (source unique),
	 * renvoie le nouvel état (chaînes hex Orca, triangles subdivisés compris).
	 */
	async stroke(
		objectId: string,
		modelId: string,
		channel: PaintChannel,
		facets: Record<string, string>,
		stroke: EngineStroke
	): Promise<Record<string, string>> {
		await this.open(objectId, modelId);
		const res = await this.request({
			type: 'stroke',
			object_id: objectId,
			channel,
			facets,
			stroke: { splitting: true, highlight_deg: 0, ...stroke }
		});
		return (res.facets ?? {}) as Record<string, string>;
	}

	/** Pinceau de hauteur de couche (`adjust_layer_height_profile`). */
	async layerAdjust(body: {
		model_id: string;
		values: Record<string, unknown>;
		matrix?: number[];
		profile: number[];
		z: number;
		delta: number;
		band_width: number;
		action: LayerEditAction;
	}): Promise<number[]> {
		const res = await this.request({ type: 'layer_adjust', ...body });
		return (res.profile ?? []) as number[];
	}

	close(): void {
		this.socket?.close();
	}
}
