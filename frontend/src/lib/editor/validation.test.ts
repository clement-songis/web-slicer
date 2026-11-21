import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { setLocale } from '$lib/i18n';
import catalog from './validation-messages.fr.json';
import {
	buildNotices,
	conversionCause,
	fillTemplate,
	issuesByObject,
	noticeSummary,
	optionKind,
	optionPage,
	plateBlocked,
	translateOrcaMessage,
	type ValidateResponse
} from './validation';

const resp = (plates: ValidateResponse['plates'], warnings: ValidateResponse['warnings'] = []) =>
	({ valid: plates.every((p) => p.valid), plates, warnings }) as ValidateResponse;
const plate = (i: number, errors: unknown[] = [], warnings: unknown[] = []) =>
	({
		plate_index: i,
		valid: errors.length === 0,
		errors,
		warnings
	}) as unknown as ValidateResponse['plates'][number];

// Messages attendus en français (langue des tests) ; dès le chargement, car
// `buildNotices` est appelé à la collecte du describe.
setLocale('fr');
beforeAll(() => setLocale('fr'));
afterAll(() => setLocale('en'));

describe('translateOrcaMessage (catalogue officiel d’Orca)', () => {
	it('gabarit boost %1% : nom de l’objet capturé', () => {
		expect(
			translateOrcaMessage(
				'cube20.stl is too close to others, and collisions may be caused.',
				catalog
			)
		).toBe('cube20.stl est trop proche des autres, cela pourrait provoquer des collisions.');
		expect(
			translateOrcaMessage('The object Tour exceeds the maximum build volume height.', catalog)
		).toBe('L’objet Tour dépasse la hauteur maximale du volume d’impression.');
	});
	it('msgid exact et guillemets', () => {
		expect(
			translateOrcaMessage('The prime tower is not supported in "By object" print.', catalog)
		).toMatch(/^La tour d’amorçage n'est pas prise en charge/);
		expect(translateOrcaMessage('Loading of a model file failed.', catalog)).toBe(
			'Le chargement du fichier modèle a échoué.'
		);
	});
	it('gabarit printf %d / %s (message GUI du type de plateau)', () => {
		const msg =
			"Plate 2: Cool Plate is not suggested to be used to print filament 1 (Bambu PLA Basic). If you still want to do this print job, please set this filament's bed temperature to non-zero.";
		const fr = translateOrcaMessage(msg, catalog);
		expect(fr).toContain('Plaque 2');
		expect(fr).toContain('Cool Plate');
		expect(fr).toContain('Bambu PLA Basic');
	});
	it('message composé : fragment traduit, nom conservé', () => {
		expect(
			translateOrcaMessage(
				'cube is too close to exclusion area, there may be collisions when printing.',
				catalog
			)
		).not.toMatch(/too close/);
	});
	it('inconnu ou déjà français : inchangé', () => {
		expect(translateOrcaMessage('plateau vide (aucun objet à trancher)', catalog)).toBe(
			'plateau vide (aucun objet à trancher)'
		);
		expect(translateOrcaMessage('Something new', catalog)).toBe('Something new');
		expect(translateOrcaMessage('Loading of a model file failed.', null)).toBe(
			'Loading of a model file failed.'
		);
	});
	it('fillTemplate : positionnel et séquentiel', () => {
		expect(fillTemplate('%2% puis %1%', ['a', 'b'])).toBe('b puis a');
		expect(fillTemplate('%s et %d', ['x', '3'])).toBe('x et 3');
	});
});

describe('options en cause', () => {
	it('type de preset comme resolve_validation_option_type', () => {
		expect(optionKind('layer_height')).toBe('process');
		expect(optionKind('enable_prime_tower')).toBe('process');
		expect(optionKind('nozzle_diameter')).toBe('machine');
		expect(optionKind('filament_flow_ratio')).toBe('filament');
		expect(optionKind('not_an_option')).toBeNull();
	});
	it('page qui contient l’option', () => {
		expect(optionPage('layer_height', 'process')).toBe('Quality');
	});
});

describe('buildNotices', () => {
	const r = resp(
		[
			plate(
				0,
				[
					{
						kind: 'object_collision_in_seq_print',
						message: 'cube20.stl is too close to others, and collisions may be caused.',
						object_id: 'old-1',
						object_name: 'cube20.stl'
					}
				],
				[
					{
						kind: 'general',
						message: 'Layer height cannot exceed nozzle diameter.',
						opt_key: 'layer_height'
					}
				]
			),
			plate(1, [{ kind: 'empty_plate', message: 'plateau vide (aucun objet à trancher)' }])
		],
		[{ key: 'post_process', message: 'scripts de post-traitement ignorés' }]
	);
	const notices = buildNotices(r, {
		catalog,
		liveId: (id) => (id === 'old-1' ? 'live-1' : undefined),
		plateCount: 2
	});
	it('erreurs puis avertissements, plateau vide non notifié', () => {
		expect(notices.map((n) => n.level)).toEqual(['error', 'warning', 'warning']);
		expect(notices.some((n) => n.kind === 'empty_plate')).toBe(false);
	});
	it('objet vivant, action Disposer, préfixe de plateau', () => {
		expect(notices[0]).toMatchObject({
			objectId: 'live-1',
			objectName: 'cube20.stl',
			action: 'arrange'
		});
		expect(notices[0].text).toMatch(/^Plateau 1 : cube20\.stl est trop proche/);
	});
	it('option : clé, type et libellé', () => {
		expect(notices[1]).toMatchObject({
			optKey: 'layer_height',
			optKind: 'process',
			optLabel: 'Layer height'
		});
		expect(notices[1].text).toMatch(/hauteur de la couche/i);
	});
	it('avertissement de configuration (racine), sans option inconnue', () => {
		expect(notices[2]).toMatchObject({
			kind: 'config_warning',
			plateIndex: null,
			optKey: undefined
		});
		expect(notices[2].text).toBe('Scripts de post-traitement ignorés');
	});
	it('surlignage et résumé', () => {
		expect(issuesByObject(notices)).toEqual({
			'live-1': { level: 'error', text: notices[0].text }
		});
		expect(noticeSummary(notices)).toBe('1 erreur · 2 avertissements');
		expect(plateBlocked(r, 0)).toBe(true);
		expect(plateBlocked(r, 1)).toBe(false);
		expect(plateBlocked(null, 0)).toBe(false);
	});
});

describe('conversionCause', () => {
	it('traduit la cause du moteur, repli si absente', () => {
		expect(conversionCause('Loading of a model file failed.', catalog)).toBe(
			'Le chargement du fichier modèle a échoué.'
		);
		expect(conversionCause('Failed loading the input model: STL tronqué', catalog)).toBe(
			'Échec du chargement du modèle : STL tronqué'
		);
		expect(conversionCause(undefined, catalog)).toMatch(/maillage/);
	});
});
