//! Preuve de substituabilité du moteur FFI (T011 × R1) : l'implémentation
//! `FfiEngine` passe intégralement la suite générique du trait `SlicerEngine`.
//! Exécution : `cargo test -p engine --features ffi --test ffi_trait_suite`.

#![cfg(feature = "ffi")]

mod common;

use common::trait_suite;
use engine::adapters::ffi::FfiEngine;

#[test]
fn ffi_engine_respecte_le_contrat() {
    trait_suite::run_all(&FfiEngine);
}
