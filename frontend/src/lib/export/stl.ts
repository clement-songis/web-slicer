// Nom de fichier sûr pour les téléchargements. L'export STL / DRC est calculé
// côté serveur par libslic3r (POST /api/projects/{id}/export/{stl,drc},
// Plater::export_stl d'Orca) ; l'ancien encodeur STL TS a été retiré.

/** Nom de fichier sûr (caractères interdits remplacés). */
export function safeFileName(name: string, extension: string): string {
	const base =
		name
			.replace(/\.[a-z0-9]+$/i, '')
			.replace(/[\\/:*?"<>|]+/g, '_')
			.trim() || 'objet';
	return `${base}.${extension}`;
}
