// Production (adapter-node, `node build`) : relaie `/api/*` vers le backend axum,
// comme le proxy Vite en dev (même origine → cookie de session conservé).
// BACKEND_URL désigne le backend (défaut http://127.0.0.1:8080, comme en dev).
// L'upgrade WebSocket `/api/ws` ne passe pas par SvelteKit : il est relayé par
// `server.js` (point d'entrée de production), ou par le reverse proxy.
import type { Handle } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

const HOP_BY_HOP = [
	'connection',
	'keep-alive',
	'transfer-encoding',
	'upgrade',
	'host',
	'content-length'
];

// Contrôle CSRF de SvelteKit (respond.js), réappliqué aux seules routes
// SvelteKit : formulaires cross-origin refusés sauf origines de confiance
// (`CSRF_TRUSTED_ORIGINS`, liste séparée par des virgules).
const FORM_TYPES = ['application/x-www-form-urlencoded', 'multipart/form-data', 'text/plain'];
const UNSAFE = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);

export function csrfForbidden(
	request: Request,
	origin: string,
	trusted: readonly string[]
): boolean {
	if (!UNSAFE.has(request.method)) return false;
	const type = (request.headers.get('content-type') ?? '').split(';', 1)[0].trim().toLowerCase();
	if (!FORM_TYPES.includes(type)) return false;
	const requestOrigin = request.headers.get('origin');
	return requestOrigin !== origin && (!requestOrigin || !trusted.includes(requestOrigin));
}

function trustedOrigins(): string[] {
	return (env.CSRF_TRUSTED_ORIGINS ?? '')
		.split(',')
		.map((o) => o.trim())
		.filter(Boolean);
}

export const handle: Handle = async ({ event, resolve }) => {
	if (!event.url.pathname.startsWith('/api/')) {
		if (csrfForbidden(event.request, event.url.origin, trustedOrigins())) {
			const message = `Cross-site ${event.request.method} form submissions are forbidden`;
			return event.request.headers.get('accept') === 'application/json'
				? new Response(JSON.stringify({ message }), {
						status: 403,
						headers: { 'content-type': 'application/json' }
					})
				: new Response(message, { status: 403 });
		}
		return resolve(event);
	}
	const backend = (env.BACKEND_URL ?? 'http://127.0.0.1:8080').replace(/\/$/, '');
	const target = backend + event.url.pathname + event.url.search;
	const headers = new Headers(event.request.headers);
	for (const h of HOP_BY_HOP) headers.delete(h);
	headers.set('x-forwarded-host', event.url.host);
	headers.set('x-forwarded-proto', event.url.protocol.replace(':', ''));
	const method = event.request.method;
	const hasBody = method !== 'GET' && method !== 'HEAD';
	let upstream: Response;
	try {
		upstream = await fetch(target, {
			method,
			headers,
			body: hasBody ? event.request.body : undefined,
			redirect: 'manual',
			// Corps en flux (uploads de modèles) : requis par undici.
			...(hasBody ? { duplex: 'half' } : {})
		} as RequestInit);
	} catch {
		return new Response(JSON.stringify({ error: 'backend injoignable' }), {
			status: 502,
			headers: { 'content-type': 'application/json' }
		});
	}
	const out = new Headers(upstream.headers);
	out.delete('content-encoding');
	out.delete('content-length');
	out.delete('transfer-encoding');
	return new Response(upstream.body, {
		status: upstream.status,
		statusText: upstream.statusText,
		headers: out
	});
};
