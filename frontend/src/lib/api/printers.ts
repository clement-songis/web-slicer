// Appels de l'API imprimantes Moonraker (T075/T077, US8), typés sur les DTO
// générés. Aucune logique métier ici — juste le transport (réducteur d'état
// dans `lib/printers/printers.ts`).
import { api } from './client';
import type {
	PrinterCatalogVendor,
	PrinterResponse,
	PrinterStatusResponse,
	PrinterUploadResponse,
	SavePrinterRequest,
	TestPrinterResponse,
	UploadToPrinterRequest,
	BonjourReply,
	ExcludeObjectStatus
} from './types';

/** Imprimantes déclarées du compte. */
export const listPrinters = () => api.get<PrinterResponse[]>('/printers');

/** Catalogue des modèles sélectionnables (marque → modèle → buses) pour le
 *  wizard et la page de gestion (Phase 14). */
export const getPrinterCatalog = () => api.get<PrinterCatalogVendor[]>('/printer-catalog');

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

/** Réponse de `POST /printers/{id}/upload-sliced` (« Send all » d'Orca) : un
 *  .gcode.3mf si l'imprimante l'accepte (`use_3mf`), sinon un G-code par plateau. */
export interface UploadSlicedResponse {
	format: 'gcode_3mf' | 'gcode';
	files: PrinterUploadResponse[];
}

/** Envoie tous les plateaux tranchés d'un projet (jobs réussis de `POST /slice`). */
export const uploadSliced = (
	id: string,
	body: { project_id: string; job_ids: string[]; start_now: boolean }
) => api.post<UploadSlicedResponse>(`/printers/${id}/upload-sliced`, body);

/** Met l'impression en pause. */
export const pausePrinter = (id: string) => api.post<void>(`/printers/${id}/pause`);

/** Reprend l'impression. */
export const resumePrinter = (id: string) => api.post<void>(`/printers/${id}/resume`);

/** Annule l'impression. */
export const cancelPrinter = (id: string) => api.post<void>(`/printers/${id}/cancel`);

// — Contrôles étendus de l'onglet Appareil (parité Device d'Orca). Routes
//   proposées au backend (relais Moonraker) ; tant qu'elles n'existent pas,
//   l'UI les détecte (404/405) et désactive les contrôles correspondants.

/** Fichier présent sur l'imprimante (`GET /server/files/list?root=gcodes`). */
export interface PrinterFile {
	path: string;
	size: number;
	/** Horodatage Unix (s). */
	modified: number;
}

/** Fichiers G-code de l'imprimante (relais `/server/files/list`). */
export const listPrinterFiles = (id: string) => api.get<PrinterFile[]>(`/printers/${id}/files`);

/** Lance l'impression d'un fichier déjà présent (relais `/printer/print/start`). */
export const startPrinterFile = (id: string, filename: string) =>
	api.post<void>(`/printers/${id}/print`, { filename });

/** Exécute un script G-code (relais `/printer/gcode/script`) : consignes de
 *  température, déplacements, mise à l'origine, ventilateurs. */
export const sendPrinterGcode = (id: string, script: string) =>
	api.post<void>(`/printers/${id}/gcode`, { script });

/** Destinations d'envoi proposées par l'hôte (stockages PrusaLink/PrusaConnect,
 *  groupes Repetier) ; `kind: 'none'` pour les autres hôtes. */
export interface HostStorages {
	kind: 'storage' | 'group' | 'none';
	items: { path: string; name: string }[];
}
export const getPrinterStorages = (id: string) => api.get<HostStorages>(`/printers/${id}/storages`);

/** Imprimantes d'un serveur Repetier (slugs à reporter dans `host_port`). */
export const getHostPrinters = (id: string) => api.get<string[]>(`/printers/${id}/host-printers`);

/** Recherche mDNS / DNS-SD d'hôtes sur le réseau local du serveur (Orca BonjourDialog). */
export const discoverPrinters = (service: string, timeoutMs = 4000) =>
	api.get<BonjourReply[]>(
		`/printers/discover?service=${encodeURIComponent(service)}&timeout_ms=${timeoutMs}`
	);

/** Objets étiquetés de l'impression en cours (Klipper exclude_object). */
export const getPrintObjects = (id: string) =>
	api.get<ExcludeObjectStatus>(`/printers/${id}/objects`);

/** Exclut un objet de l'impression en cours (EXCLUDE_OBJECT, irréversible ; 409 hors impression). */
export const excludePrintObject = (id: string, name: string) =>
	api.post<ExcludeObjectStatus>(`/printers/${id}/objects/exclude`, { name });
