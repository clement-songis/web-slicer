// Identité applicative, affichée dans l'UI et les exports 3MF/G-code.
export const APP_NAME = 'Web-Slicer';

/** Nom + version courte pour les métadonnées de fichiers générés. */
export function appIdentity(version: string): string {
	return `${APP_NAME}/${version}`;
}
