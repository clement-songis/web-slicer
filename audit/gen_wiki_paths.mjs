#!/usr/bin/env node
// Phase 34 : chemins d'aide wiki par option (Orca 2.4.1 Tab.cpp
// `append_single_option_line(key, label_path)`, `line.label_path`) →
// frontend/src/lib/settings/wiki-paths.ts. Lien = OptionsGroup::get_url :
// https://www.orcaslicer.com/wiki/<label_path>. Première occurrence par clé
// (TabPrint avant TabPrintModel, filament, machine). Usage :
//   node audit/gen_wiki_paths.mjs && (cd frontend && npx prettier --write src/lib/settings/wiki-paths.ts)
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = readFileSync(join(root, 'vendor/OrcaSlicer/src/slic3r/GUI/Tab.cpp'), 'utf8');
const paths = new Map();
const set = (key, path) => {
	if (key && path && !paths.has(key)) paths.set(key, path);
};
const vars = new Map();
let linePath = null;
for (const raw of src.split('\n')) {
	const line = raw.replace(/\/\/.*$/, '');
	let m;
	const opt = /Option\s+(\w+)\s*=\s*optgroup->get_option\("([\w]+)"/.exec(line);
	if (opt) vars.set(opt[1], opt[2]);
	const re = /append_single_option_line\(\s*(?:"([\w#]+)"|(\w+))\s*,\s*"([^"]+)"/g;
	while ((m = re.exec(line))) set(m[1] ?? vars.get(m[2]), m[3]);
	const lp = /\bline\.label_path\s*=\s*"([^"]+)"/.exec(line);
	if (lp) linePath = lp[1];
	const ap = /line\.append_option\(\s*optgroup->get_option\("([\w]+)"/.exec(line);
	if (ap && linePath) set(ap[1], linePath);
	if (/append_line\(\s*line\s*\)/.test(line)) linePath = null;
}
const keys = [...paths.keys()].sort();
const body = keys.map((k) => `\t${JSON.stringify(k)}: ${JSON.stringify(paths.get(k))}`).join(',\n');
const out = `// GÉNÉRÉ par audit/gen_wiki_paths.mjs depuis vendor/OrcaSlicer/src/slic3r/GUI/Tab.cpp
// (label_path des lignes d'options) — ne pas éditer à la main ; relancer le script (puis prettier).
// ${keys.length} options.
export const WIKI_PATHS: Readonly<Record<string, string>> = {
${body}
};
`;
writeFileSync(join(root, 'frontend/src/lib/settings/wiki-paths.ts'), out);
console.log(`wiki-paths.ts : ${keys.length} options`);
