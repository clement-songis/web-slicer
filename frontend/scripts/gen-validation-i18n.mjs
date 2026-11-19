#!/usr/bin/env node
// Catalogue FR des messages de validation d'Orca (phase 29) : chaînes
// traduisibles (L(…), _L(…), _u8L(…)) des sources qui produisent les erreurs /
// avertissements de POST /api/projects/{id}/validate et les causes d'échec de
// conversion, traduites par le catalogue officiel d'OrcaSlicer
// (localization/i18n/fr/OrcaSlicer_fr.po). Seules les entrées traduites sont
// gardées ; les gabarits (%1%, %s, %d…) sont résolus côté client
// (translateOrcaMessage, src/lib/editor/validation.ts).
//   ORCA_SRC=… node scripts/gen-validation-i18n.mjs   (dans nix develop)
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const FRONT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ORCA = process.env.ORCA_SRC || join(FRONT, '../vendor/OrcaSlicer');

// Sources (et début) d'où viennent les messages :
// - Print.cpp : Print::validate, collisions, filaments, tour d'amorçage ;
// - PrintObject.cpp / GCode.cpp : erreurs de tranchage par objet ;
// - PrintConfig.cpp à partir de DynamicPrintConfig::validate (le début du
//   fichier = libellés d'options, traduits par ailleurs) ;
// - Model.cpp, Format/* : chargement d'un modèle (cause de conversion) ;
// - Plater.cpp : réécriture GUI du message « type de plateau / filament ».
const lib = join(ORCA, 'src/libslic3r');
const sources = [
	{ file: join(lib, 'Print.cpp') },
	{ file: join(lib, 'PrintObject.cpp') },
	{ file: join(lib, 'GCode.cpp') },
	{ file: join(lib, 'Model.cpp') },
	{
		file: join(lib, 'PrintConfig.cpp'),
		from: 'std::map<std::string, std::string> DynamicPrintConfig::validate',
		to: 'CLIActionsConfigDef::CLIActionsConfigDef()'
	},
	...readdirSync(join(lib, 'Format'))
		.filter((f) => f.endsWith('.cpp'))
		.map((f) => ({ file: join(lib, 'Format', f) })),
	{ file: join(ORCA, 'src/slic3r/GUI/Plater.cpp'), from: 'post_process_string_object_exception' }
];

const ESC = { n: '\n', t: '\t', '"': '"', "'": "'", '\\': '\\' };
const unescapeCpp = (s) => s.replace(/\\(n|t|"|'|\\)/g, (_, c) => ESC[c]);
const msgids = new Set();
const CALL = /(?<![A-Za-z0-9_])(?:_u8L|_L|L)\(\s*((?:"(?:[^"\\]|\\.)*"\s*)+)\)/g;
for (const { file, from, to } of sources) {
	let text = readFileSync(file, 'utf8');
	if (from) {
		const at = text.indexOf(from);
		if (at < 0) throw new Error(from + ' introuvable dans ' + file);
		text = text.slice(at);
	}
	if (to && text.includes(to)) text = text.slice(0, text.indexOf(to));
	for (const m of text.matchAll(CALL)) {
		const pieces = [...m[1].matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((p) => unescapeCpp(p[1]));
		const id = pieces.join('');
		if (id.trim().length >= 4) msgids.add(id);
	}
}

// Parse minimal du .po (cf. gen-settings-i18n.mjs).
const po = new Map();
const unq = (s) => JSON.parse(s);
let cur = null;
let field = null;
const flush = () => {
	if (cur && !cur.ctx && cur.id && cur.str && !cur.fuzzy) po.set(cur.id, cur.str);
};
const poText = readFileSync(join(ORCA, 'localization/i18n/fr/OrcaSlicer_fr.po'), 'utf8');
for (const raw of poText.split('\n')) {
	const line = raw.trim();
	if (line.startsWith('#,') && line.includes('fuzzy')) {
		flush();
		cur = { id: '', str: '', ctx: false, fuzzy: true };
		field = null;
	} else if (line.startsWith('msgctxt')) {
		if (!cur || cur.id) {
			flush();
			cur = { id: '', str: '', ctx: false, fuzzy: false };
		}
		cur.ctx = true;
		field = null;
	} else if (line.startsWith('msgid ')) {
		if (!cur || cur.id || cur.str) {
			flush();
			cur = {
				id: '',
				str: '',
				ctx: cur?.ctx && !cur.id ? true : false,
				fuzzy: !!(cur?.fuzzy && !cur.id)
			};
		}
		cur.id = unq(line.slice(6));
		field = 'id';
	} else if (line.startsWith('msgid_plural')) {
		field = null;
	} else if (line.startsWith('msgstr ')) {
		cur.str = unq(line.slice(7));
		field = 'str';
	} else if (line.startsWith('msgstr[0] ')) {
		cur.str = unq(line.slice(10));
		field = 'str';
	} else if (line.startsWith('"') && cur && field) {
		cur[field] += unq(line);
	}
}
flush();

const catalog = {};
const missing = [];
for (const id of [...msgids].sort()) {
	const tr = po.get(id);
	if (tr && tr !== id) catalog[id] = tr;
	else if (!tr) missing.push(id);
}
const OUT = join(FRONT, 'src/lib/editor/validation-messages.fr.json');
writeFileSync(OUT, JSON.stringify(catalog, null, '\t') + '\n');
console.log(
	JSON.stringify({
		msgids: msgids.size,
		translated: Object.keys(catalog).length,
		untranslated: missing.length,
		sample: missing.slice(0, 12)
	})
);
