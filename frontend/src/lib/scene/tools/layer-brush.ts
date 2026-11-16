// Pinceau de hauteur de couche, parité GLCanvas3D (LayersEditing) : chaque
// évènement souris (appui, glisser) applique `adjust_layer_height_profile` avec
// `strength` = 0,005 à la cote pointée, et un minuteur de 300 ms répète l'action
// tant que le bouton reste enfoncé (`_perform_layer_editing_action` →
// `_start_timer` → `on_timer`). Le calcul est fait par le moteur ; ce module
// ne fait que séquencer les évènements (une requête à la fois, file bornée).
// Pur (minuteur injectable) → testable.

export type LayerBrushAction = 'increase' | 'decrease' | 'reduce' | 'smooth';

/** `LayersEditing::strength` d'Orca. */
export const ORCA_LAYER_STRENGTH = 0.005;
/** Période du minuteur d'Orca (`m_timer.Start(300)`). */
export const ORCA_LAYER_TIMER_MS = 300;
/** File maximale (évènements en attente pendant un calcul moteur). */
export const MAX_QUEUED = 8;

/** Action d'Orca selon les boutons : Maj → (droit ? smooth : reduce), sinon
 *  (droit ? increase : decrease). */
export function actionFor(rightButton: boolean, shift: boolean): LayerBrushAction {
	if (shift) return rightButton ? 'smooth' : 'reduce';
	return rightButton ? 'increase' : 'decrease';
}

export interface Timer {
	set(fn: () => void, ms: number): unknown;
	clear(handle: unknown): void;
}

const defaultTimer: Timer = {
	set: (fn, ms) => setInterval(fn, ms),
	clear: (h) => clearInterval(h as ReturnType<typeof setInterval>)
};

export class LayerBrushDriver {
	private z = 0;
	private action: LayerBrushAction = 'decrease';
	private timer: unknown = null;
	private queued = 0;
	private chain: Promise<void> = Promise.resolve();

	constructor(
		private readonly apply: (z: number, action: LayerBrushAction, delta: number) => Promise<void>,
		private readonly clock: Timer = defaultTimer
	) {}

	get active(): boolean {
		return this.timer !== null;
	}

	/** Appui : action immédiate puis répétition toutes les 300 ms. */
	start(z: number, action: LayerBrushAction): void {
		this.stop();
		this.z = z;
		this.action = action;
		this.fire();
		this.timer = this.clock.set(() => this.fire(), ORCA_LAYER_TIMER_MS);
	}

	/** Glisser : nouvelle cote + action immédiate. */
	move(z: number): void {
		if (!this.active) return;
		this.z = z;
		this.fire();
	}

	stop(): void {
		if (this.timer !== null) this.clock.clear(this.timer);
		this.timer = null;
	}

	/** Attend la fin des requêtes en cours (tests, fermeture). */
	idle(): Promise<void> {
		return this.chain;
	}

	private fire(): void {
		if (this.queued >= MAX_QUEUED) return;
		this.queued++;
		const z = this.z;
		const action = this.action;
		this.chain = this.chain
			.then(() => this.apply(z, action, ORCA_LAYER_STRENGTH))
			.catch(() => this.stop())
			.finally(() => {
				this.queued--;
			});
	}
}
