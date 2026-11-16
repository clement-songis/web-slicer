// Valeurs Orca `coPoint*` : forme texte `XxY` (ex. `170x170`, `-3x4.5`).
// Les points vectoriels sont passés au widget une entrée à la fois par OptionLine.

/** Affiche une valeur de point sous la forme canonique attendue par Orca. */
export function formatPoint(value: unknown): string {
	if (typeof value === 'string') return value.replace('×', 'x').replace(/\s+/g, '');
	if (Array.isArray(value) && value.length >= 2) {
		const x = Number(value[0]);
		const y = Number(value[1]);
		if (Number.isFinite(x) && Number.isFinite(y)) return `${x}x${y}`;
	}
	return '';
}

/** Parse `XxY`; renvoie null pendant une saisie incomplète ou invalide. */
export function parsePoint(raw: string): string | null {
	const match = raw
		.trim()
		.replace('×', 'x')
		.match(/^(-?(?:\d+(?:\.\d*)?|\.\d+))\s*x\s*(-?(?:\d+(?:\.\d*)?|\.\d+))$/i);
	if (!match) return null;
	return `${Number(match[1])}x${Number(match[2])}`;
}
