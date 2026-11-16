// Copie les modèles de calibration OrcaSlicer (resources/calib/*/*.{drc,3mf}) vers
// `static/calib/` en conservant l'arborescence (menu « Calibration » : tour de
// température, tour de pressure advance…). Lancé au dev/build ; destination
// gitignorée (dérivée de vendor/OrcaSlicer ou de $ORCA_SRC, lecture seule).
// Source absente → no-op silencieux (comme copy-printer-covers.mjs).
import { cp, mkdir, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const candidates = [
	join(here, '../../vendor/OrcaSlicer/resources/calib'),
	process.env.ORCA_SRC ? join(process.env.ORCA_SRC, 'resources/calib') : null
].filter(Boolean);
const src = candidates.find((c) => existsSync(c));
const dest = join(here, '../static/calib');

if (!src) {
	console.warn('[calib] modèles de calibration Orca absents — copie ignorée');
	process.exit(0);
}

let n = 0;
for (const dir of await readdir(src, { withFileTypes: true })) {
	if (!dir.isDirectory()) continue;
	for (const f of await readdir(join(src, dir.name))) {
		if (!/\.(?:drc|3mf)$/i.test(f)) continue;
		await mkdir(join(dest, dir.name), { recursive: true });
		await cp(join(src, dir.name, f), join(dest, dir.name, f));
		n += 1;
	}
}
console.log(`[calib] ${n} modèles copiés vers static/calib/`);
