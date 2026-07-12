// Copie les vignettes d'imprimante (`*_cover.png`) des profils vendeurs
// OrcaSlicer vers `static/printers/`, en préservant l'arborescence
// `<Marque>/<Modèle>_cover.png` (Phase 14, T134). Lancé au dev/build : le
// dossier de destination est gitignoré (~11 Mo, dérivé de vendor/OrcaSlicer,
// source unique en lecture seule). Absence des profils → no-op silencieux.
import { cp, mkdir, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const profiles = join(here, '../../vendor/OrcaSlicer/resources/profiles');
const dest = join(here, '../static/printers');

async function* walkCovers(dir) {
	for (const entry of await readdir(dir, { withFileTypes: true })) {
		const full = join(dir, entry.name);
		if (entry.isDirectory()) {
			yield* walkCovers(full);
		} else if (entry.isFile() && entry.name.toLowerCase().endsWith('_cover.png')) {
			yield full;
		}
	}
}

if (!existsSync(profiles)) {
	console.warn(`[covers] profils absents (${profiles}) — copie ignorée`);
	process.exit(0);
}

let n = 0;
for await (const src of walkCovers(profiles)) {
	const out = join(dest, relative(profiles, src));
	await mkdir(dirname(out), { recursive: true });
	await cp(src, out);
	n += 1;
}
console.log(`[covers] ${n} vignettes copiées vers static/printers/`);
