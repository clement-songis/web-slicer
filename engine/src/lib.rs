//! Moteur de slicing — trait `SlicerEngine` et registre de paramètres.
//!
//! Constitution II : ce crate encapsule libslic3r d'OrcaSlicer ; le backend
//! ne dépend que du trait (défini ici, T011) et jamais des adaptateurs
//! (`adapters::ffi` — bridge cxx principal ; `adapters::cli` — fallback).

pub mod params;
