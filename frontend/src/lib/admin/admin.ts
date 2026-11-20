// Logique pure de la page Administration (phase 32) : politique d'inscription,
// limite d'upload et quotas en Mo, lien d'invitation, mot de passe généré,
// pagination de la liste des comptes (GET /admin/users, lot 32).

export type RegistrationPolicy = 'open' | 'invite' | 'closed';

export const POLICIES: { value: RegistrationPolicy; label: string; hint: string }[] = [
	{ value: 'open', label: 'Ouvertes', hint: 'Tout visiteur peut créer un compte.' },
	{
		value: 'invite',
		label: 'Sur invitation',
		hint: "Un jeton d'invitation (lien ci-dessous) est exigé à l'inscription."
	},
	{ value: 'closed', label: 'Fermées', hint: "Seul l'administrateur crée les comptes." }
];

const MB = 1024 * 1024;

/** Octets → Mo affichés (entier si rond, sinon 1 décimale). */
export function bytesToMb(bytes: number | bigint): number {
	const mb = Number(bytes) / MB;
	return Number.isInteger(mb) ? mb : Math.round(mb * 10) / 10;
}

/** Mo saisis → octets (entier positif) ou `null` si invalide. */
export function mbToBytes(mb: number): number | null {
	if (!Number.isFinite(mb) || mb <= 0) return null;
	return Math.round(mb * MB);
}

/** Quota en Mo saisi → octets ; 0 (ou vide) = illimité ; null si invalide. */
export function quotaMbToBytes(mb: number | null | undefined): number | null {
	if (mb === null || mb === undefined || (typeof mb === 'number' && Number.isNaN(mb))) return 0;
	if (!Number.isFinite(mb) || mb < 0) return null;
	return Math.round(mb * MB);
}

/** Tranchages simultanés saisis (0 = illimité, 0..=1000) ; null si invalide. */
export function parseSliceLimit(v: number | null | undefined): number | null {
	if (v === null || v === undefined || Number.isNaN(v)) return 0;
	return Number.isInteger(v) && v >= 0 && v <= 1000 ? v : null;
}

/** Libellé de pagination : « 51–100 sur 132 ». */
export function pageLabel(page: number, perPage: number, total: number): string {
	if (total === 0) return 'Aucun compte';
	const from = (page - 1) * perPage + 1;
	const to = Math.min(total, page * perPage);
	return `${from}–${to} sur ${total}`;
}

/** Lien d'inscription porteur du jeton (lu par /register?invite=). */
export function invitationLink(origin: string, token: string): string {
	return `${origin.replace(/\/$/, '')}/register?invite=${encodeURIComponent(token)}`;
}

/** Mot de passe aléatoire lisible (sans caractères ambigus), 14 caractères. */
export function generatePassword(random: (n: number) => Uint8Array = defaultRandom): string {
	const alphabet = 'abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789';
	return Array.from(random(14), (b) => alphabet[b % alphabet.length]).join('');
}

function defaultRandom(n: number): Uint8Array {
	const out = new Uint8Array(n);
	crypto.getRandomValues(out);
	return out;
}
