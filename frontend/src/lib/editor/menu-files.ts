// Extraction des archives du menu Fichier. `fflate` est également utilisé par
// l'écosystème Three.js : décompression ZIP locale, sans envoyer l'archive à un
// service tiers. Seuls les fichiers réellement importables sont retournés.
import { unzipSync } from 'fflate';
import { tr } from '#lib/i18n';

const MODEL_EXTENSION = /\.(stl|obj|3mf|oltp|step|stp|amf|svg|drc)$/i;
const CONFIG_EXTENSION = /\.(json|orca_printer|orca_bundle|orca_filament)$/i;
const MAX_ARCHIVE_BYTES = 512 * 1024 * 1024;

async function archiveEntries(file: File): Promise<Record<string, Uint8Array>> {
	try {
		return unzipSync(new Uint8Array(await file.arrayBuffer()), {
			filter: (entry) => !entry.name.endsWith('/') && entry.originalSize <= MAX_ARCHIVE_BYTES
		});
	} catch {
		throw new Error(tr('Unreadable ZIP archive: {file}', { file: file.name }));
	}
}

function checkedEntries(entries: Record<string, Uint8Array>, source: string) {
	const files = Object.entries(entries);
	if (
		files.length > 1000 ||
		files.reduce((sum, [, data]) => sum + data.length, 0) > MAX_ARCHIVE_BYTES
	)
		throw new Error(tr('ZIP archive too large: {file}', { file: source }));
	return files;
}

/** Modèles contenus dans un ZIP, dans l'ordre de l'archive. */
export async function modelsFromZip(file: File): Promise<File[]> {
	const entries = checkedEntries(await archiveEntries(file), file.name);
	return entries
		.filter(([name]) => MODEL_EXTENSION.test(name))
		.map(([name, data]) => {
			const copy = new Uint8Array(data);
			return new File([copy], name.split('/').pop() ?? name);
		});
}

export interface PresetDocument {
	kind: string;
	name: string;
	inherits?: string;
	values: Record<string, unknown>;
}

function presetDocument(value: unknown, fallbackName: string): PresetDocument {
	if (!value || typeof value !== 'object' || Array.isArray(value))
		throw new Error(tr('Invalid JSON configuration: {file}', { file: fallbackName }));
	const source = value as Record<string, unknown>;
	const rawKind = typeof source.type === 'string' ? source.type : 'process';
	const kind = rawKind === 'print' ? 'process' : rawKind;
	if (!['process', 'filament', 'machine', 'machine_model'].includes(kind))
		throw new Error(tr('Unknown preset type: {kind}', { kind: String(rawKind) }));
	const name = typeof source.name === 'string' && source.name.trim() ? source.name : fallbackName;
	const values = { ...source };
	for (const key of [
		'type',
		'name',
		'inherits',
		'from',
		'setting_id',
		'instantiation',
		'version',
		'url',
		'description',
		'filament_id'
	])
		delete values[key];
	return {
		kind,
		name,
		...(typeof source.inherits === 'string' ? { inherits: source.inherits } : {}),
		values
	};
}

async function configDocuments(file: File): Promise<PresetDocument[]> {
	const fallback = file.name.replace(/\.[^.]+$/, '');
	if (file.name.toLowerCase().endsWith('.json'))
		return [presetDocument(JSON.parse(await file.text()), fallback)];
	if (!/\.(zip|orca_printer|orca_bundle|orca_filament)$/i.test(file.name)) return [];
	const entries = checkedEntries(await archiveEntries(file), file.name);
	const out: PresetDocument[] = [];
	for (const [name, data] of entries) {
		if (!CONFIG_EXTENSION.test(name) || !name.toLowerCase().endsWith('.json')) continue;
		const text = new TextDecoder().decode(data);
		out.push(
			presetDocument(
				JSON.parse(text),
				name
					.split('/')
					.pop()
					?.replace(/\.json$/i, '') ?? name
			)
		);
	}
	return out;
}

/** Profils Orca contenus dans des JSON/ZIP/bundles sélectionnés ensemble. */
export async function presetsFromFiles(files: File[]): Promise<PresetDocument[]> {
	const groups = await Promise.all(files.map(configDocuments));
	return groups.flat();
}
