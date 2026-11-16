#!/usr/bin/env node
// Génère les messages i18n FR des titres de pages, sections, libellés,
// infobulles et valeurs d'énumération des options du dialogue de réglages, à partir de la traduction officielle
// d'OrcaSlicer (localization/i18n/fr/OrcaSlicer_fr.po). Les clés restent les
// libellés anglais d'Orca (index inverse de lib/i18n) : rien ne change côté
// clés Orca ni data-testid. Ajout seulement : un message déjà présent (même
// texte anglais) n'est jamais réécrit.
//   ORCA_SRC=… node scripts/gen-settings-i18n.mjs   (dans nix develop)
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const FRONT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ORCA = process.env.ORCA_SRC;
if (!ORCA) throw new Error('ORCA_SRC absent : lancer dans nix develop');

const params = {};
for (const line of readFileSync(join(FRONT, 'src/generated/params.ts'), 'utf8').split('\n')) {
	const m = line.match(/^\s*"([^"]+)": (\{.*\}),?$/);
	if (m) params[m[1]] = JSON.parse(m[2]);
}
const layoutSrc = readFileSync(join(FRONT, 'src/generated/ui-layout.ts'), 'utf8');
const start = layoutSrc.indexOf('[', layoutSrc.indexOf('export const UI_LAYOUT:'));
const end = layoutSrc.indexOf('\n];', start) + 2;
const layout = new Function('return ' + layoutSrc.slice(start, end))();

const labels = new Set();
for (const page of layout) {
	labels.add(page.title === 'page_name' || page.title === 'Extruder' ? 'Extruder 1' : page.title);
	for (const section of page.sections) {
		if (section.title) labels.add(section.title);
		for (const opt of section.options) {
			if (typeof opt !== 'string' || !params[opt]) continue;
			const def = params[opt];
			if (def.label) labels.add(def.label);
			if (def.tooltip) labels.add(def.tooltip);
			for (const e of def.enumLabels ?? []) if (e) labels.add(e);
		}
	}
}

// Parse minimal du .po : entrées sans msgctxt, msgid/msgstr mono ou multi-lignes.
const po = new Map();
const unq = (s) => JSON.parse(s);
let cur = null;
let field = null;
const flush = () => {
	if (cur && !cur.ctx && cur.id && cur.str && !cur.fuzzy) po.set(cur.id, cur.str);
};
for (const raw of readFileSync(join(ORCA, 'localization/i18n/fr/OrcaSlicer_fr.po'), 'utf8').split(
	'\n'
)) {
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

// Sortie : dictionnaire paresseux src/lib/settings/i18n/fr.json (« anglais →
// français »), importé à l'ouverture du dialogue de réglages — hors du chunk
// initial (Paraglide compilait ≈1100 messages dans chaque page). Les anciens
// messages `set_*` éventuellement présents dans messages/*.json sont retirés ;
// les messages Paraglide existants (menus…) restent prioritaires et intacts.
const read = (f) => JSON.parse(readFileSync(join(FRONT, 'messages', f), 'utf8'));
const en = read('en.json');
const fr = read('fr.json');
let stripped = 0;
for (const id of Object.keys(en))
	if (id.startsWith('set_')) {
		delete en[id];
		delete fr[id];
		stripped++;
	}
const existing = new Set(
	Object.entries(en)
		.filter(([k]) => k !== '$schema')
		.map(([, v]) => v)
);
const dict = { 'Extruder 1': 'Extrudeur 1' };
const missing = [];
for (const label of [...labels].sort()) {
	if (existing.has(label) || label in dict) continue;
	const tr = po.get(label);
	if (!tr) {
		missing.push(label);
		continue;
	}
	// Les infobulles multi-lignes gardent leurs retours ; les libellés courts
	// sont normalisés sur une ligne.
	dict[label] = label.includes('\n') ? tr.trim() : tr.replace(/\s+/g, ' ').trim();
}
const dump = (o) => JSON.stringify(o, null, '\t') + '\n';
writeFileSync(join(FRONT, 'messages/en.json'), dump(en));
writeFileSync(join(FRONT, 'messages/fr.json'), dump(fr));
mkdirSync(join(FRONT, 'src/lib/settings/i18n'), { recursive: true });
writeFileSync(join(FRONT, 'src/lib/settings/i18n/fr.json'), dump(dict));
console.log(
	JSON.stringify({
		labels: labels.size,
		poEntries: po.size,
		dictionary: Object.keys(dict).length,
		strippedFromMessages: stripped,
		untranslated: missing.length,
		sample: missing.slice(0, 15)
	})
);
