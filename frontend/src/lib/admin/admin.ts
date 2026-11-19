// Logique pure de la page Administration (phase 32) : politique d'inscription,
// limite d'upload en Mo, lien d'invitation, mot de passe généré, liste locale des
// comptes gérés (le serveur n'expose pas encore la liste des comptes).

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

/** Compte géré depuis cette page (mémorisé dans le navigateur de l'admin). */
export interface ManagedUser {
	id: string;
	email: string;
	role: string;
}

/** Ajoute ou remplace un compte (par id), le plus récent en tête. */
export function upsertManaged(list: readonly ManagedUser[], user: ManagedUser): ManagedUser[] {
	return [user, ...list.filter((u) => u.id !== user.id)];
}

/** Relit la liste mémorisée (tolère une valeur absente ou corrompue). */
export function parseManaged(raw: string | null): ManagedUser[] {
	if (!raw) return [];
	try {
		const v = JSON.parse(raw) as unknown;
		return Array.isArray(v)
			? v.filter(
					(u): u is ManagedUser => !!u && typeof u.id === 'string' && typeof u.email === 'string'
				)
			: [];
	} catch {
		return [];
	}
}
