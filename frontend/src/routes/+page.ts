// La racine renvoie vers la bibliothèque ; la garde de `library` redirige vers
// `/login` si la session est absente.
import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = () => {
	redirect(307, '/library');
};
