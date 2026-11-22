// Clic droit de la vue 3D (audit 46 PREF-c) : un glisser au bouton droit qui
// déplace la caméra (panoramique par défaut, ou « Pivoter ») n'ouvre pas le
// menu contextuel au relâchement — Orca GLCanvas3D::on_mouse
// (`m_mouse.ignore_right_up = true` dès que la caméra bouge, GLCanvas3D.cpp:4599
// et :4625 ; menu posté seulement si `!ignore_right_up`, :4726 ; remis à zéro
// au relâchement, :5522). Bouton droit réglé sur « Aucun » ou navigation
// « pavé tactile » : la caméra ne bouge pas, le menu s'ouvre comme dans Orca.
//
// Selon le navigateur, `contextmenu` arrive à l'appui (Linux, macOS) ou au
// relâchement (Windows) : à l'appui, l'ouverture est différée au relâchement
// et abandonnée si le pointeur a bougé entre-temps.

export class RightClickGuard {
	private down = false;
	private moved = false;
	private startX = 0;
	private startY = 0;
	private pending: (() => void) | null = null;

	/** `cameraMoves` : le glisser droit déplace-t-il la caméra ? */
	constructor(private readonly cameraMoves: () => boolean) {}

	pointerDown(button: number, x: number, y: number) {
		if (button !== 2) return;
		this.down = true;
		this.moved = false;
		this.startX = x;
		this.startY = y;
		this.pending = null;
	}

	pointerMove(buttons: number, x: number, y: number) {
		if (!this.down || (buttons & 2) === 0) return;
		// Tout déplacement fait bouger la caméra d'Orca (évènement Dragging).
		if ((x !== this.startX || y !== this.startY) && this.cameraMoves()) {
			this.moved = true;
			this.pending = null;
		}
	}

	pointerUp(button: number) {
		if (button !== 2 || !this.down) return;
		this.down = false;
		const open = this.pending;
		this.pending = null;
		if (open && !this.moved) open();
	}

	/**
	 * Évènement `contextmenu` : `open` est appelé tout de suite (relâchement
	 * déjà passé, sans glisser), différé au relâchement (bouton encore
	 * enfoncé), ou jamais (la caméra a bougé).
	 */
	contextMenu(open: () => void): void {
		if (this.moved) {
			// Relâchement déjà passé (Windows) : le glisser est consommé.
			if (!this.down) this.moved = false;
			return;
		}
		if (this.down) {
			this.pending = open;
			return;
		}
		open();
	}
}
