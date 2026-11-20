// Lien d'aide wiki d'une option (Orca OG_CustomCtrl::CtrlLine::launch_browser
// → OptionsGroup::get_url : https://www.orcaslicer.com/wiki/<label_path>), au
// clic sur le libellé. Chemins issus de Tab.cpp (wiki-paths.ts, généré).
import { WIKI_PATHS } from './wiki-paths';

export const ORCA_WIKI_BASE = 'https://www.orcaslicer.com/wiki/';

/** URL du wiki d'Orca pour une option, ou `null` si Orca n'en donne pas. */
export function wikiUrl(key: string): string | null {
	const path = WIKI_PATHS[key];
	return path ? ORCA_WIKI_BASE + path : null;
}
