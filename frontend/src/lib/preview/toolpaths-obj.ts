// Export « toolpaths as OBJ » (menu Fichier › Export) : trajets d'extrusion de
// l'aperçu en OBJ (sommets `v` + segments `l`), déplacements exclus. Orca
// exporte des tubes maillés ; ici des polylignes, lisibles par tout outil OBJ.
import type { PreviewSegments } from './decode';

/** Rôle « Travel » (`SEGMENT_ROLE_COLORS`). */
const TRAVEL = 20;

export function toolpathsToObj(
	chunks: readonly PreviewSegments[],
	header = 'web-slicer toolpaths'
): string {
	const out: string[] = [`# ${header}`];
	const edges: string[] = [];
	let vertex = 0;
	for (const s of chunks) {
		for (let i = 0; i < s.count; i++) {
			if (s.kind[i] === TRAVEL) continue;
			const a = s.start.subarray(i * 3, i * 3 + 3);
			const b = s.end.subarray(i * 3, i * 3 + 3);
			out.push(`v ${a[0]} ${a[1]} ${a[2]}`, `v ${b[0]} ${b[1]} ${b[2]}`);
			edges.push(`l ${vertex + 1} ${vertex + 2}`);
			vertex += 2;
		}
	}
	return [...out, ...edges, ''].join('\n');
}
