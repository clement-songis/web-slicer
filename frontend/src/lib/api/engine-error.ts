// Messages d'erreur serveur lisibles (phase 38). Certains 422 du moteur
// remontent la sortie brute du worker libslic3r (« lecture libslic3r : worker
// sorti en code 1 — stderr :  — stdout : E invalid_model The supplied file
// couldn't be read because it's empty ») : on garde le contexte du serveur
// (« 3MF projet illisible ») et on remplace le détail technique par une cause
// compréhensible. Les messages sans trace de worker sont rendus tels quels.

/** Causes connues (code du worker / texte d'Orca) → explication. */
const CAUSES: [RegExp, string][] = [
	[/because it'?s empty|file is empty|empty file/i, 'le fichier ne contient aucun objet'],
	[
		/no (?:printable )?objects?|does not contain any (?:valid )?(?:object|geometry)/i,
		'le fichier ne contient aucun objet'
	],
	[
		/invalid_model|could ?n[o']t be read|unable to read|failed to read/i,
		'le fichier est endommagé ou n’est pas un modèle valide'
	],
	[/unsupported|not supported/i, 'format non pris en charge'],
	[/timed? ?out|timeout/i, 'le traitement a pris trop de temps'],
	[/out of memory|bad_alloc/i, 'mémoire insuffisante pour ce modèle']
];

const WORKER_TRACE =
	/(?:\s*:\s*)?(?:lecture libslic3r\s*:\s*)?worker sorti en code\s*-?\d+[\s\S]*$/i;

/** Vrai si le message contient la sortie brute du worker moteur. */
export function isTechnicalEngineMessage(message: string): boolean {
	return /worker sorti en code|stdout\s*:|stderr\s*:/i.test(message);
}

/** Message lisible (inchangé s'il ne contient pas de trace du worker). */
export function readableServerMessage(message: string): string {
	if (!message || !isTechnicalEngineMessage(message)) return message;
	const cause = CAUSES.find(([re]) => re.test(message))?.[1] ?? 'le moteur n’a pas pu le lire';
	const context = message
		.replace(WORKER_TRACE, '')
		.replace(/\s*[:—-]\s*$/, '')
		.trim();
	const head = context || 'Fichier illisible';
	return `${head} : ${cause}.`;
}
