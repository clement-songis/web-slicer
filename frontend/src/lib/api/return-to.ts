// Retour à la page demandée après connexion (phase 32) : une page protégée
// ouverte sans session redirige vers /login?next=<chemin> ; après connexion on y
// revient au lieu de la page de démarrage. Seuls les chemins internes sont admis
// (pas de redirection ouverte vers un autre site).

/** Chemin interne sûr, ou `null`. */
export function safeNext(raw: string | null | undefined): string | null {
	if (!raw) return null;
	if (!raw.startsWith('/') || raw.startsWith('//') || raw.startsWith('/\\')) return null;
	if (/^\/(login|register)(\/|\?|$)/.test(raw)) return null;
	return raw;
}

/** URL de connexion qui ramène à `path` (chemin + requête courants). */
export function loginUrl(path: string | null | undefined): string {
	const next = safeNext(path);
	return next && next !== '/' ? `/login?next=${encodeURIComponent(next)}` : '/login';
}
