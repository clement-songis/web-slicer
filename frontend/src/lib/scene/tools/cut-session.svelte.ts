// État de l'outil Coupe (phase 28) partagé entre le panneau (CutTool), la vue
// 3D (plan incliné, anneaux de rotation, connecteurs cliqués sur la face de
// coupe) et la page (requête serveur). La géométrie est dans cut-plan.ts.
import {
	AXIS_ROTATION,
	axisOfRotation,
	boundsOf,
	boxCenter,
	buildCutRequest,
	centeredWorld,
	connectorAt,
	cutBlocker,
	defaultConnectorParams,
	defaultGroove,
	dominantAxis,
	normalizeConnector,
	offsetRange,
	planeBasis,
	planePoint,
	planeQuaternion,
	planeSize,
	pointToUv,
	sectionSegments,
	uvToPoint,
	wrapDeg,
	type Axis,
	type ConnectorDraft,
	type ConnectorParams,
	type CutDraft,
	type CutMode,
	type GrooveDraft,
	type ObjectXf,
	type PartAction,
	type Vec3
} from './cut-plan';
import type { CutRequest } from '../../../generated/api/CutRequest';
import { untrack } from 'svelte';

export interface CutTarget {
	id: string;
	positions: ArrayLike<number>;
	indices: ArrayLike<number>;
	xf: ObjectXf;
}

/** Plan dessiné par la vue 3D (repère monde). */
export interface CutGuide {
	center: Vec3;
	normal: Vec3;
	quaternion: [number, number, number, number];
	size: [number, number];
	/** Rotation Orca du plan (degrés) : état initial des anneaux. */
	rotation: Vec3;
	connectors: {
		id: string;
		center: Vec3;
		size: number;
		depth: number;
		segments: number;
		selected: boolean;
		rotation: number;
	}[];
	/** Clic sur le plan (mode connecteurs) : point monde. */
	onclick?: (world: Vec3) => void;
	/** Anneaux de rotation : nouvelle rotation (degrés, ordre Orca). */
	onrotate?: (rot: Vec3) => void;
}

const SEGMENTS = { triangle: 3, square: 4, hexagon: 6, circle: 32 } as const;
let nextId = 1;

/** État de l'outil annulable (plan, connecteurs, réglages) — sans sélection ni mode d'édition. */
interface CutToolState {
	mode: CutMode;
	rotation: Vec3;
	offset: number;
	upper: PartAction;
	lower: PartAction;
	asParts: boolean;
	connectors: ConnectorDraft[];
	params: ConnectorParams;
	groove: GrooveDraft | null;
}
/** Pas d'annulation regroupés : les changements continus (glisser du plan, d'un connecteur,
 *  des anneaux) séparés de moins de ce délai forment une seule étape. */
const HISTORY_BURST_MS = 400;
const HISTORY_MAX = 100;

export class CutSession {
	target = $state.raw<CutTarget | null>(null);
	mode = $state<CutMode>('planar');
	rotation = $state<Vec3>([0, 0, 0]);
	offset = $state(0);
	upper = $state<PartAction>({ keep: true, placeOnCut: true, flip: false });
	lower = $state<PartAction>({ keep: true, placeOnCut: false, flip: false });
	asParts = $state(false);
	connectors = $state<ConnectorDraft[]>([]);
	editingConnectors = $state(false);
	selectedId = $state<string | null>(null);
	params = $state<ConnectorParams>(defaultConnectorParams());
	groove = $state<GrooveDraft | null>(null);

	// — Annuler / rétablir propres à l'outil (audit 40 B2) — Orca prend un
	//   instantané SnapshotType::GizmoAction à chaque action du gizmo de coupe
	//   (« Add connector », « Move connector », « Move cut plane »…,
	//   GLGizmoCut.cpp) et Ctrl+Z, outil ouvert, ne remonte que ces étapes.
	private undoStack: string[] = [];
	private redoStack: string[] = [];
	private committed: string | null = null;
	private burstUntil = 0;
	private restoring = false;
	/** Nombre d'étapes annulables / rétablissables (affichage, tests). */
	undoCount = $state(0);
	redoCount = $state(0);

	/** Sommets dans le repère C (monde centré sur l'objet). */
	readonly cpos = $derived(
		this.target ? centeredWorld(this.target.positions, this.target.xf) : new Float32Array()
	);
	readonly box = $derived(boundsOf(this.cpos));
	readonly center = $derived(boxCenter(this.box));
	readonly basis = $derived(planeBasis(this.rotation));
	readonly range = $derived(offsetRange(this.cpos, this.center, this.basis.n));
	readonly origin = $derived(planePoint(this.center, this.basis.n, this.offset));
	readonly axis = $derived<Axis | null>(axisOfRotation(this.rotation));
	readonly section = $derived(
		this.target
			? sectionSegments(this.cpos, this.target.indices, this.origin, this.basis)
			: { segments: [], min: [0, 0] as [number, number], max: [0, 0] as [number, number] }
	);
	readonly draft = $derived<CutDraft>({
		mode: this.mode,
		rotation: this.rotation,
		offset: this.offset,
		upper: this.upper,
		lower: this.lower,
		asParts: this.asParts,
		connectors: this.connectors,
		groove: this.groove
	});
	readonly blocker = $derived(cutBlocker(this.draft, this.range));
	/** Axe affiché (« Hauteur » si la normale est surtout verticale). */
	readonly displayAxis = $derived(dominantAxis(this.basis.n));

	/** Change d'objet : plan horizontal au centre, connecteurs effacés. */
	bind(target: CutTarget | null) {
		// Appelé depuis un effet de la page : lecture non suivie (sinon boucle).
		untrack(() => {
			const changed = target?.id !== this.target?.id;
			this.target = target;
			if (changed) {
				this.reset();
				this.clearHistory();
			}
		});
	}

	/** Empreinte de l'état annulable (lue dans un effet : dépendances suivies). */
	historyKey(): string {
		const s: CutToolState = {
			mode: this.mode,
			rotation: this.rotation,
			offset: this.offset,
			upper: this.upper,
			lower: this.lower,
			asParts: this.asParts,
			connectors: this.connectors,
			params: this.params,
			groove: this.groove
		};
		return JSON.stringify(s);
	}

	/**
	 * Note l'état courant (appelé par un effet du panneau à chaque changement) :
	 * le premier changement d'une rafale empile l'état d'avant ; les suivants
	 * (glisser continu) prolongent la même étape.
	 */
	noteChange(key: string, now = Date.now()) {
		if (this.restoring) {
			this.committed = key;
			return;
		}
		if (this.committed === null) {
			this.committed = key;
			return;
		}
		if (key === this.committed) return;
		if (now > this.burstUntil) {
			this.undoStack.push(this.committed);
			if (this.undoStack.length > HISTORY_MAX) this.undoStack.shift();
			this.redoStack = [];
		}
		this.burstUntil = now + HISTORY_BURST_MS;
		this.committed = key;
		this.syncCounts();
	}

	/** Annule la dernière étape de l'outil ; faux s'il n'y en a plus. */
	undo(): boolean {
		const prev = this.undoStack.pop();
		if (prev === undefined) return false;
		this.redoStack.push(this.historyKey());
		this.restore(prev);
		return true;
	}

	/** Rétablit l'étape annulée ; faux s'il n'y en a pas. */
	redo(): boolean {
		const next = this.redoStack.pop();
		if (next === undefined) return false;
		this.undoStack.push(this.historyKey());
		this.restore(next);
		return true;
	}

	/** Oublie l'historique de l'outil (autre objet, outil fermé, coupe effectuée). */
	clearHistory() {
		this.undoStack = [];
		this.redoStack = [];
		this.committed = null;
		this.burstUntil = 0;
		this.syncCounts();
	}

	private restore(key: string) {
		const s = JSON.parse(key) as CutToolState;
		this.restoring = true;
		this.mode = s.mode;
		this.rotation = s.rotation;
		this.offset = s.offset;
		this.upper = s.upper;
		this.lower = s.lower;
		this.asParts = s.asParts;
		this.connectors = s.connectors;
		this.params = s.params;
		this.groove = s.groove;
		if (this.selectedId && !s.connectors.some((c) => c.id === this.selectedId))
			this.selectedId = null;
		this.committed = key;
		this.burstUntil = 0;
		this.restoring = false;
		this.syncCounts();
	}

	private syncCounts() {
		this.undoCount = this.undoStack.length;
		this.redoCount = this.redoStack.length;
	}

	reset() {
		this.rotation = [0, 0, 0];
		this.offset = 0;
		this.connectors = [];
		this.selectedId = null;
		this.editingConnectors = false;
		this.groove = this.target ? defaultGroove(this.box) : null;
	}

	setAxis(a: Axis) {
		this.rotation = [...AXIS_ROTATION[a]];
		this.offset = 0;
	}

	setRotation(rot: Vec3) {
		this.rotation = rot.map(wrapDeg) as Vec3;
	}

	/** Retourne le plan (Orca « Flip cut plane ») : rotation de 180° autour de X du plan. */
	flipPlane() {
		this.setRotation([this.rotation[0] + 180, this.rotation[1], this.rotation[2]]);
		this.offset = -this.offset;
	}

	/** Coordonnée monde (le long de l'axe affiché) du centre du plan. */
	worldOf(offset: number): number {
		const a = this.displayAxis;
		const p = this.target?.xf.position ?? [0, 0, 0];
		return p[a] + this.center[a] + this.basis.n[a] * offset;
	}

	offsetOf(world: number): number {
		const a = this.displayAxis;
		const p = this.target?.xf.position ?? [0, 0, 0];
		const k = this.basis.n[a] || 1;
		return (world - p[a] - this.center[a]) / k;
	}

	setMode(mode: CutMode) {
		this.mode = mode;
		if (mode === 'groove') this.editingConnectors = false;
	}

	/** Ajoute (ou sélectionne) un connecteur au point (u, v) de la face de coupe. */
	pickUv(u: number, v: number) {
		const hit = connectorAt(this.connectors, u, v);
		if (hit) {
			this.selectedId = hit.id;
			return;
		}
		const c: ConnectorDraft = { id: 'c' + nextId++, u, v, ...normalizeConnector(this.params) };
		this.connectors = [...this.connectors, c];
		this.selectedId = c.id;
	}

	/** Clic dans la vue 3D : point monde projeté sur le plan. */
	pickWorld(world: Vec3) {
		const p = this.target?.xf.position ?? [0, 0, 0];
		const [u, v] = pointToUv(this.origin, this.basis, [
			world[0] - p[0],
			world[1] - p[1],
			world[2] - p[2]
		]);
		this.pickUv(u, v);
	}

	moveConnector(id: string, u: number, v: number) {
		this.connectors = this.connectors.map((c) => (c.id === id ? { ...c, u, v } : c));
	}

	removeConnector(id: string | null = this.selectedId) {
		this.connectors = this.connectors.filter((c) => c.id !== id);
		if (this.selectedId === id) this.selectedId = null;
	}

	clearConnectors() {
		this.connectors = [];
		this.selectedId = null;
	}

	/** Réglages courants : appliqués au connecteur sélectionné, ou à tous s'il n'y en a pas. */
	setParams(patch: Partial<ConnectorParams>) {
		this.params = normalizeConnector({ ...this.params, ...patch });
		const apply = (c: ConnectorDraft) => ({ ...c, ...this.params, id: c.id, u: c.u, v: c.v });
		this.connectors = this.connectors.map((c) =>
			this.selectedId === null || c.id === this.selectedId ? apply(c) : c
		);
	}

	select(id: string | null) {
		this.selectedId = id;
		const c = this.connectors.find((k) => k.id === id);
		if (c) {
			const { id: _i, u: _u, v: _v, ...rest } = c;
			void _i;
			void _u;
			void _v;
			this.params = rest;
		}
	}

	request(instanceMatrix: () => number[]): CutRequest | null {
		if (!this.target) return null;
		return buildCutRequest(this.draft, this.center, this.target.xf, instanceMatrix);
	}

	guide(): CutGuide | null {
		const t = this.target;
		if (!t) return null;
		const p = t.xf.position;
		const o = this.origin;
		const world = (q: Vec3): Vec3 => [q[0] + p[0], q[1] + p[1], q[2] + p[2]];
		return {
			center: world(o),
			normal: this.basis.n,
			quaternion: planeQuaternion(this.rotation),
			size: planeSize(this.box, this.basis),
			rotation: this.rotation,
			connectors:
				this.mode === 'planar'
					? this.connectors.map((c) => ({
							id: c.id,
							center: world(uvToPoint(o, this.basis, [c.u, c.v])),
							size: c.size,
							depth: c.depth,
							segments: SEGMENTS[c.shape],
							rotation: c.rotation,
							selected: c.id === this.selectedId
						}))
					: [],
			onclick:
				this.editingConnectors && this.mode === 'planar' ? (w) => this.pickWorld(w) : undefined,
			onrotate: (rot) => this.setRotation(rot)
		};
	}
}
