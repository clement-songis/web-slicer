// Nom de projet par défaut remplacé par le premier modèle importé (Orca
// `Plater::add_model`/`load_files` : si le projet s'appelle « Untitled », il
// prend le nom — sans extension — du premier fichier chargé).

/** Noms donnés aux projets créés sans nom (bibliothèque, menu Fichier, Orca). */
export const DEFAULT_PROJECT_NAMES: readonly string[] = [
	'Sans titre',
	'Nouveau projet',
	'New Project',
	'Untitled'
];

export function isDefaultProjectName(name: string): boolean {
	return DEFAULT_PROJECT_NAMES.includes(name.trim());
}

/** Nom de projet tiré d'un fichier : `path.stem()` (dernière extension retirée). */
export function projectNameFromFile(fileName: string): string {
	const base = fileName.split(/[\\/]/).pop() ?? fileName;
	const dot = base.lastIndexOf('.');
	return (dot > 0 ? base.slice(0, dot) : base).trim();
}
