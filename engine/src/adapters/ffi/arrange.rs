//! Arrangement et orientation automatiques via libslic3r (T016, FR-013).

use std::collections::BTreeMap;

use crate::api::{
    ArrangeParams, BuildVolume, EngineError, EngineErrorCode, EngineResult, Model, ModelObject,
};

use super::bridge::ffi;
use super::model::{ffi_guard, to_object, to_raw_objects};

/// Arrangement réel par le nester de libslic3r, plateau par plateau.
///
/// Les exclusions sont transportées dans `bed_xy` après le contour du plateau,
/// séparées par un point `[NaN, NaN]`. Ce marqueur est interne au bridge (les
/// coordonnées valides sont finies) et évite d'élargir l'ABI cxx historique.
/// Chaque appel arrange uniquement les instances d'un `plate_index` : les
/// plateaux virtuels créés par le nester sont refusés par `apply_arrange_polys`,
/// et les indices de plateau d'origine sont conservés côté Rust.
///
/// Limites du contrat actuel : aucun plateau supplémentaire n'est créé pour les
/// objets qui ne tiennent pas ; `max_height` et `use_machine_clearance` ne sont
/// pas exposés par `arrange_raw` et ne peuvent donc pas être transmis à Orca.
/// L'alignement final automatique est désactivé dans le bridge, car il ignore
/// les obstacles préchargés et peut les faire chevaucher après le nesting —
/// sauf en `orca_cli_mode`, qui reproduit exactement l'arrangement du CLI Orca
/// (alignement final CENTER, exclusions gonflées de 1 mm).
pub fn arrange(model: &mut Model, bed: &BuildVolume, params: &ArrangeParams) -> EngineResult<()> {
    arrange_ordered(model, bed, params).map(|_| ())
}

/// Comme [`arrange`] ; renvoie en plus, en arrangement séquentiel, l'ordre
/// d'impression de chaque instance (ordre objets/instances, 1-based par
/// plateau, rang de `ModelInstance::arrange_order` d'Orca), sinon `None`.
pub fn arrange_ordered(
    model: &mut Model,
    bed: &BuildVolume,
    params: &ArrangeParams,
) -> EngineResult<Vec<Option<u32>>> {
    validate_arrange_input(bed, params)?;
    let instance_total: usize = model.objects.iter().map(|o| o.instances.len()).sum();
    let mut orders: Vec<Option<u32>> = vec![None; instance_total];
    if instance_total == 0 {
        return Ok(orders);
    }
    // Index plat (objet, instance) → position dans `orders`.
    let mut flat_index = BTreeMap::new();
    for (o, object) in model.objects.iter().enumerate() {
        for i in 0..object.instances.len() {
            let n = flat_index.len();
            flat_index.insert((o, i), n);
        }
    }
    let orca_config = (params.print_config.is_some() || params.is_seq_print).then(|| {
        super::model::config_to_orca_json(&params.print_config.clone().unwrap_or_default())
    });

    let _guard = ffi_guard();
    let bed_xy = encode_bed_and_exclusions(bed);
    // Commit uniquement si tous les plateaux ont été arrangés : une erreur sur
    // un plateau ultérieur ne laisse pas une scène à moitié déplacée.
    let mut output = model.clone();

    // Regroupe sans changer l'ordre objets/instances : le bridge renvoie les
    // matrices dans ce même ordre et Rust les réapplique sur la scène d'origine.
    let mut plates: BTreeMap<u32, BTreeMap<usize, Vec<usize>>> = BTreeMap::new();
    for (object_index, object) in model.objects.iter().enumerate() {
        for (instance_index, instance) in object.instances.iter().enumerate() {
            plates
                .entry(instance.plate_index)
                .or_default()
                .entry(object_index)
                .or_default()
                .push(instance_index);
        }
    }

    for (plate_index, objects) in plates {
        let mut plate_model = Model::default();
        for (&object_index, instance_indices) in &objects {
            let mut object = model.objects[object_index].clone();
            object.instances = instance_indices
                .iter()
                .map(|&index| object.instances[index].clone())
                .collect();
            plate_model.objects.push(object);
        }

        let raw = to_raw_objects(&plate_model);
        let plate_error = |e: cxx::Exception| {
            EngineError::new(
                EngineErrorCode::OutOfBuildVolume,
                format!("plateau {plate_index} : {e}"),
            )
        };
        let (arranged, plate_orders) = match &orca_config {
            Some(json) => {
                let r = ffi::arrange_orca_raw(
                    &raw,
                    &bed_xy,
                    json,
                    params.is_seq_print,
                    params.allow_rotations,
                )
                .map_err(plate_error)?;
                let ranks = r.is_seq_print.then(|| print_ranks(&r.arrange_order));
                (r.objects, ranks)
            }
            None => (
                ffi::arrange_raw(
                    &raw,
                    &bed_xy,
                    params.clearance,
                    params.allow_rotations,
                    params.orca_cli_mode,
                )
                .map_err(plate_error)?,
                None,
            ),
        };
        if let Some(ranks) = &plate_orders {
            let mut k = 0;
            for (&object_index, instance_indices) in &objects {
                for &instance_index in instance_indices {
                    orders[flat_index[&(object_index, instance_index)]] = ranks.get(k).copied();
                    k += 1;
                }
            }
        }
        if arranged.len() != objects.len() {
            return Err(EngineError::new(
                EngineErrorCode::Internal,
                "l'arrangeur a modifié le nombre d'objets",
            ));
        }

        for (raw_object, (&object_index, instance_indices)) in
            arranged.into_iter().zip(objects.iter())
        {
            if raw_object.instances.len() != instance_indices.len() {
                return Err(EngineError::new(
                    EngineErrorCode::Internal,
                    "l'arrangeur a modifié le nombre d'instances",
                ));
            }
            for (raw_instance, &instance_index) in
                raw_object.instances.into_iter().zip(instance_indices)
            {
                output.objects[object_index].instances[instance_index].matrix =
                    glam::DMat4::from_cols_array(&raw_instance.matrix);
            }
        }
    }
    *model = output;
    Ok(orders)
}

/// Rangs 1-based croissants de `arrange_order` (ordre de placement du nester,
/// trié comme GCode::_do_export / Print::sequential_print_clearance_valid).
fn print_ranks(arrange_order: &[i32]) -> Vec<u32> {
    let mut idx: Vec<usize> = (0..arrange_order.len()).collect();
    idx.sort_by_key(|&i| (arrange_order[i], i));
    let mut ranks = vec![0u32; arrange_order.len()];
    for (rank, i) in idx.into_iter().enumerate() {
        ranks[i] = rank as u32 + 1;
    }
    ranks
}

fn validate_arrange_input(bed: &BuildVolume, params: &ArrangeParams) -> EngineResult<()> {
    let valid_polygon = |polygon: &[[f64; 2]]| {
        polygon.len() >= 3
            && polygon
                .iter()
                .all(|point| point[0].is_finite() && point[1].is_finite())
    };
    if !valid_polygon(&bed.bed_shape) || !bed.excluded.iter().all(|p| valid_polygon(p)) {
        return Err(EngineError::new(
            EngineErrorCode::OutOfBuildVolume,
            "contour de plateau ou exclusion invalide",
        ));
    }
    if !params.clearance.is_finite() || params.clearance < 0.0 {
        return Err(EngineError::new(
            EngineErrorCode::OutOfBuildVolume,
            "dégagement d'arrangement invalide",
        ));
    }
    Ok(())
}

fn encode_bed_and_exclusions(bed: &BuildVolume) -> Vec<f64> {
    let mut out: Vec<f64> = bed.bed_shape.iter().flatten().copied().collect();
    for excluded in &bed.excluded {
        out.extend_from_slice(&[f64::NAN, f64::NAN]);
        out.extend(excluded.iter().flatten().copied());
    }
    out
}

pub fn orient(object: &mut ModelObject) -> EngineResult<()> {
    let _guard = ffi_guard();
    let mut model = Model {
        objects: vec![object.clone()],
        custom_gcodes: Vec::new(),
    };
    let raw = to_raw_objects(&model);
    let oriented = ffi::orient_raw(&raw[0])
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, e.to_string()))?;
    model.objects = vec![to_object(oriented)];
    *object = model.objects.remove(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bed_encoding_separates_exclusions() {
        let bed = BuildVolume {
            bed_shape: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]],
            max_height: 200.0,
            excluded: vec![vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0]]],
        };
        let encoded = encode_bed_and_exclusions(&bed);
        assert_eq!(&encoded[..6], &[0.0, 0.0, 100.0, 0.0, 100.0, 100.0]);
        assert!(encoded[6].is_nan() && encoded[7].is_nan());
        assert_eq!(&encoded[8..], &[10.0, 10.0, 20.0, 10.0, 20.0, 20.0]);
    }

    #[test]
    fn rejects_non_finite_clearance_before_ffi() {
        let bed = BuildVolume {
            bed_shape: vec![[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]],
            max_height: 200.0,
            excluded: vec![],
        };
        let params = ArrangeParams {
            clearance: f64::NAN,
            use_machine_clearance: false,
            allow_rotations: false,
            orca_cli_mode: false,
            print_config: None,
            is_seq_print: false,
        };
        assert!(arrange(&mut Model::default(), &bed, &params).is_err());
    }
}
