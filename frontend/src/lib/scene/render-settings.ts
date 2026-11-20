// Graphismes (phase 34, Orca Preferences › Graphismes, version navigateur) :
// rendu WebGL créé selon les préférences — anticrénelage MSAA (fixé à la
// création du contexte : la vue est recréée quand il change), plafond
// d'images/s (Orca `max_fps`) appliqué à la boucle d'animation de Threlte
// (`renderer.setAnimationLoop`), et compteur d'images réellement dessinées.
import { WebGLRenderer } from 'three';
import { appPreferences } from '$lib/settings/preferences';

/** Marge sous l'intervalle cible : le rafraîchissement d'écran oscille de ~1 ms. */
const JITTER_MS = 2;

/**
 * Faut-il exécuter cette image ? `cap` = images/s maximum (0 = illimité).
 * Une image sautée n'est pas perdue : la demande de rendu de Threlte reste
 * en attente et part à l'image autorisée suivante.
 */
export function shouldRunFrame(now: number, last: number, cap: number): boolean {
	if (!(cap > 0)) return true;
	return now - last >= 1000 / cap - JITTER_MS;
}

/** Compteur d'images dessinées (lu et remis à zéro par l'affichage). */
export class FrameMeter {
	frames = 0;
	private since: number | null = null;
	/** Images/s depuis la lecture précédente (`null` à la première lecture). */
	sample(now: number): number | null {
		const since = this.since;
		const frames = this.frames;
		this.since = now;
		this.frames = 0;
		if (since === null || now <= since) return null;
		return Math.round((frames * 1000) / (now - since));
	}
}

let fpsCap = 0;
appPreferences.subscribe((p) => {
	fpsCap = p.graphics?.fpsCap ?? 0;
});

type LoopCallback = Parameters<WebGLRenderer['setAnimationLoop']>[0];

/**
 * Fabrique de rendu pour `<Canvas createRenderer>` : mêmes options que le
 * rendu par défaut de Threlte, MSAA selon `antialias`, plafond d'images/s
 * lu en continu dans les préférences, images comptées dans `meter`.
 */
export function rendererFactory(antialias: boolean, meter: FrameMeter) {
	return (canvas: HTMLCanvasElement): WebGLRenderer => {
		const renderer = new WebGLRenderer({
			canvas,
			powerPreference: 'high-performance',
			antialias,
			alpha: true
		});
		const setLoop = renderer.setAnimationLoop.bind(renderer);
		let last = -Infinity;
		renderer.setAnimationLoop = (callback: LoopCallback) => {
			if (!callback) return setLoop(null);
			setLoop((time, frame) => {
				if (!shouldRunFrame(time, last, fpsCap)) return;
				last = time;
				callback(time, frame);
			});
		};
		const render = renderer.render.bind(renderer);
		renderer.render = (scene, camera) => {
			meter.frames += 1;
			render(scene, camera);
		};
		return renderer;
	};
}
