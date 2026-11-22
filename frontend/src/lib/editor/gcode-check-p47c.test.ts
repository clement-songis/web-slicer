// Lot 47c (audit 45 H2D-a) : comme l'interface d'Orca, un tranchage dont les
// trajets sortent de la zone d'une buse réussit (aperçu gardé) ; l'erreur
// vient de `stats.gcode_check` et de /validate (`kind` = `gcode_check`), elle
// bloque export / envoi mais pas le tranchage.
import { describe, expect, it } from 'vitest';
import { gcodeCheckError, isNozzleRangeError } from './nozzle-range-error';
import { buildNotices, plateBlocked, type ValidateResponse } from './validation';

const ORCA =
	'Filament 1 is placed in the right nozzle, but the generated G-code path exceeds the printable range of the right nozzle.';

describe('stats.gcode_check (lot 47c)', () => {
	it('restitue le message d’Orca, reconnu comme erreur de buse', () => {
		const text = gcodeCheckError({
			level: 'error',
			error_code: 1,
			nozzle: 'right nozzle',
			filament: [1],
			message: ORCA
		});
		expect(text).not.toBeNull();
		expect(isNozzleRangeError(text)).toBe(true);
	});

	it('absent, nul ou sans message → aucune erreur', () => {
		expect(gcodeCheckError(null)).toBeNull();
		expect(gcodeCheckError(undefined)).toBeNull();
		expect(gcodeCheckError({ nozzle: 'right nozzle' })).toBeNull();
		expect(gcodeCheckError({ message: '  ' })).toBeNull();
	});

	it('message inconnu : gardé tel quel', () => {
		expect(gcodeCheckError({ message: 'Autre contrôle' })).toBe('Autre contrôle');
	});
});

describe('/validate gcode_check (lot 47c)', () => {
	const resp = {
		plates: [
			{
				plate_index: 0,
				valid: false,
				errors: [{ kind: 'gcode_check', message: ORCA, params: ['right nozzle', '1'] }],
				warnings: []
			}
		],
		warnings: []
	} as unknown as ValidateResponse;

	it('erreur affichée, ne grise pas « Trancher le plateau »', () => {
		const notices = buildNotices(resp, { catalog: null });
		expect(notices).toHaveLength(1);
		expect(notices[0].level).toBe('error');
		expect(notices[0].kind).toBe('gcode_check');
		expect(isNozzleRangeError(notices[0].text)).toBe(true);
		expect(plateBlocked(resp, 0)).toBe(false);
	});
});
