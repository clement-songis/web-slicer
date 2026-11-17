// Génère src/generated/gcode-placeholders.ts depuis les sources d'Orca
// (libslic3r/PrintConfig.cpp) : variables propres au G-code personnalisé,
// telles que les liste EditGCodeDialog (états de tranchage, statistiques,
// objets, dimensions, températures, horodatages, variables spécifiques par clé).
// Usage : ORCA_SRC=… node scripts/gen-gcode-placeholders.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const src = readFileSync(join(process.env.ORCA_SRC, 'src/libslic3r/PrintConfig.cpp'), 'utf8');
const unquote = (expr) =>
	[...expr.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => JSON.parse('"' + m[1] + '"')).join('');

function block(cls) {
	const start = src.indexOf(cls + '::' + cls + '()');
	if (start < 0) throw new Error('classe absente : ' + cls);
	const end = src.indexOf('\n}\n', start);
	return src.slice(start, end);
}

function defs(cls) {
	const body = block(cls);
	const out = [];
	const re =
		/this->add\("([a-z0-9_]+)",\s*(co[A-Za-z]+)\)|new_def\("([a-z0-9_]+)",\s*(co[A-Za-z]+),\s*"([^"]*)",\s*(.*?)\)\s*;?\s*$/gm;
	let m;
	const starts = [];
	while ((m = re.exec(body))) starts.push({ m, at: m.index });
	starts.forEach(({ m, at }, i) => {
		if (m[3]) {
			if (m[4] === 'coNone') return;
			out.push({ key: m[3], type: m[4], label: m[5], tooltip: unquote(m[6]) });
			return;
		}
		const chunk = body.slice(at, starts[i + 1]?.at ?? body.length);
		const label = /def->label\s*=\s*L\(([\s\S]*?)\);/.exec(chunk);
		const tip = /def->tooltip\s*=\s*([\s\S]*?);\s*$/m.exec(chunk);
		out.push({
			key: m[1],
			type: m[2],
			label: label ? unquote(label[1]) : m[1],
			tooltip: tip ? unquote(tip[1]) : ''
		});
	});
	return out;
}

const groups = [
	['Slicing State (lecture seule)', 'ReadOnlySlicingStatesConfigDef'],
	['Slicing State (lecture et écriture)', 'ReadWriteSlicingStatesConfigDef'],
	['Slicing State', 'OtherSlicingStatesConfigDef'],
	['Print Statistics', 'PrintStatisticsConfigDef'],
	['Objects Info', 'ObjectsInfoConfigDef'],
	['Dimensions', 'DimensionsConfigDef'],
	['Temperatures', 'TemperaturesConfigDef'],
	['Timestamps', 'TimestampsConfigDef']
].map(([title, cls]) => ({ title, items: defs(cls) }));

const specificDefs = Object.fromEntries(
	defs('CustomGcodeSpecificConfigDef').map((d) => [d.key, d])
);
const mapStart = src.indexOf('s_CustomGcodeSpecificPlaceholders');
const mapBody = src.slice(mapStart, src.indexOf('};', mapStart));
const specific = {};
for (const m of mapBody.matchAll(/\{"([a-z_]+)",\s*\{([^}]*)\}\}/g))
	specific[m[1]] = [...m[2].matchAll(/"([a-z0-9_]+)"/g)]
		.map((k) => k[1])
		.filter((k) => specificDefs[k]);

const banner =
	'// Généré par scripts/gen-gcode-placeholders.mjs depuis OrcaSlicer PrintConfig.cpp — ne pas éditer.\n';
writeFileSync(
	new URL('../src/generated/gcode-placeholders.ts', import.meta.url),
	banner +
		'export interface GcodePlaceholder { key: string; type: string; label: string; tooltip: string }\n' +
		'export const GCODE_PLACEHOLDER_GROUPS: { title: string; items: GcodePlaceholder[] }[] = ' +
		JSON.stringify(groups, null, '\t') +
		';\n' +
		'export const GCODE_SPECIFIC_DEFS: Record<string, GcodePlaceholder> = ' +
		JSON.stringify(specificDefs, null, '\t') +
		';\n' +
		'export const GCODE_SPECIFIC_PLACEHOLDERS: Record<string, string[]> = ' +
		JSON.stringify(specific, null, '\t') +
		';\n'
);
console.log(
	groups.map((g) => g.title + ':' + g.items.length).join(' '),
	'| spécifiques',
	Object.keys(specific).length
);
