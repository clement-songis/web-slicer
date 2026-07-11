// Appels de l'API imprimantes Moonraker (T075/T077, US8), typés sur les DTO
// générés. Aucune logique métier ici — juste le transport (réducteur d'état
// dans `lib/printers/printers.ts`).
import { api } from './client';
import type {
	PrinterResponse,
	PrinterStatusResponse,
	PrinterUploadResponse,
	SavePrinterRequest,
	TestPrinterResponse,
	UploadToPrinterRequest
} from './types';

/** Imprimantes déclarées du compte. */
export const listPrinters = () => api.get<PrinterResponse[]>('/printers');

/** Déclare une imprimante (clé API chiffrée au repos côté serveur). */
export const createPrinter = (body: SavePrinterRequest) =>
	api.post<PrinterResponse>('/printers', body);

/** Met à jour une imprimante déclarée. */
export const updatePrinter = (id: string, body: SavePrinterRequest) =>
	api.put<PrinterResponse>(`/printers/${id}`, body);

/** Retire une imprimante déclarée. */
export const deletePrinter = (id: string) => api.del<void>(`/printers/${id}`);

/** Test de connexion : relaie `GET /server/info` (FR-060). */
export const testPrinter = (id: string) => api.post<TestPrinterResponse>(`/printers/${id}/test`);

/** État instantané ; amorce aussi le suivi WebSocket `printer.status` (T076). */
export const getPrinterStatus = (id: string) =>
	api.get<PrinterStatusResponse>(`/printers/${id}/status`);

/** Envoie un G-code du compte vers l'imprimante (FR-061). */
export const uploadToPrinter = (id: string, body: UploadToPrinterRequest) =>
	api.post<PrinterUploadResponse>(`/printers/${id}/upload`, body);

/** Met l'impression en pause. */
export const pausePrinter = (id: string) => api.post<void>(`/printers/${id}/pause`);

/** Reprend l'impression. */
export const resumePrinter = (id: string) => api.post<void>(`/printers/${id}/resume`);

/** Annule l'impression. */
export const cancelPrinter = (id: string) => api.post<void>(`/printers/${id}/cancel`);
