// Audit 41 V4 : noms des volumes / objets créés par les outils Texte et SVG,
// comme Orca 2.4.1 — texte : le texte lui-même, retours à la ligne remplacés
// par des espaces (GLGizmoEmboss.cpp, create_volume_name) ; SVG : nom du
// fichier sans extension, sinon « SVG shape » (GLGizmoSVG.cpp, volume_name).
import { tr } from '#lib/i18n';

export function textVolumeName(text: string): string {
	const name = text.replace(/\r?\n/g, ' ').trim();
	return name || tr('Text');
}

export function svgVolumeName(fileName?: string | null): string {
	const base = (fileName ?? '').split(/[/\\]/).pop() ?? '';
	const dot = base.lastIndexOf('.');
	const stem = (dot > 0 ? base.slice(0, dot) : base).trim();
	return stem || tr('SVG shape');
}
