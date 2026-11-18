// Précompilation des shaders (PreviewWarmup) : attend `compile` (promesse de
// `renderer.compileAsync`), bornée par un délai de garde, sans jamais rejeter
// (une compilation échouée laisse simplement le rendu normal la refaire).

/** Délai maximal avant de reprendre le rendu sans attendre la compilation. */
export const WARMUP_TIMEOUT_MS = 4000;

export async function warmupScene(
	compile: () => Promise<unknown>,
	timeoutMs = WARMUP_TIMEOUT_MS,
	now: () => number = () => performance.now()
): Promise<{ ms: number; timedOut: boolean }> {
	const start = now();
	let timer: ReturnType<typeof setTimeout> | undefined;
	const timeout = new Promise<'timeout'>((resolve) => {
		timer = setTimeout(() => resolve('timeout'), timeoutMs);
	});
	let outcome: unknown;
	try {
		// Laisse les composants frères monter leurs objets avant la compilation.
		await Promise.resolve();
		outcome = await Promise.race([compile().catch(() => 'error'), timeout]);
	} finally {
		clearTimeout(timer);
	}
	return { ms: Math.round(now() - start), timedOut: outcome === 'timeout' };
}
