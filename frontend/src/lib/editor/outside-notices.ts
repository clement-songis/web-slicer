// Audit 42 I1 — notifications « objet hors plateau », calculées dans le
// navigateur dès le déplacement (sans attendre la validation du serveur) :
//   - à cheval / trop haut : texte d'Orca 2.4.1 (construct_error_string,
//     GLCanvas3D.cpp:1411 : « Following objects are laid over the boundary of
//     plate or exceeds the height limit: … Please solve the problem… ») ;
//   - entièrement dehors : l'appartenance au plateau étant explicite ici (Orca
//     la déduit de la position et ne tranche pas l'objet), on demande de le
//     replacer.
// Les deux sont des erreurs bloquantes (« Trancher » grisé, export / envoi
// désactivés), avec l'action « Disposer ».
import { tr } from '$lib/i18n';
import type { VolumeState } from '$lib/scene/print-volume';
import type { ValidationNotice } from './validation';

export interface OutsideObject {
	id: string;
	name: string;
	state: Exclude<VolumeState, 'inside'>;
	/** Plateau (0-based). */
	plateIndex: number;
}

export function outsideNotices(
	objects: readonly OutsideObject[],
	plateCount: number
): ValidationNotice[] {
	const multi = plateCount > 1;
	return objects.map((o) => {
		const text =
			o.state === 'partly'
				? tr(
						'Following objects are laid over the boundary of plate or exceeds the height limit:\n{name}\nPlease solve the problem by moving it totally on or off the plate, and confirming that the height is within the build volume.',
						{ name: o.name }
					)
				: tr(
						'{name} is outside the plate: move it onto the plate (or onto another plate) before slicing.',
						{
							name: o.name
						}
					);
		return {
			id: 'outside:' + o.id,
			level: 'error' as const,
			kind: 'object_outside_plate' as const,
			plateIndex: o.plateIndex,
			text: multi ? tr('Plate {n}: {text}', { n: o.plateIndex + 1, text }) : text,
			objectId: o.id,
			objectName: o.name,
			action: 'arrange' as const
		};
	});
}
