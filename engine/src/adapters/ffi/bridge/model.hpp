// Wrappers C++ minces au-dessus de libslic3r (déclarations pour cxx).
#pragma once

#include "rust/cxx.h"

#include <cstddef>
#include <cstdint>
#include <string>

namespace Slic3r {
class Model;
class ModelVolume;
}

namespace webslicer {

struct RawObject;
struct RawProject;
struct RawMesh;
struct RawRepairResult;
struct RawSliceResult;

void init_runtime(rust::Str temp_dir, rust::Str data_dir);

rust::Vec<RawObject> load_model_raw(rust::Str path);
struct RawStepMesh;
RawStepMesh load_step_raw(rust::Str path, double linear, double angle, bool split_compound);

RawProject read_project_3mf_raw(rust::Str path);
// Relief ré-éditable (ModelVolume::text_configuration / emboss_shape) <-> JSON
// web-slicer ; chaîne vide = aucun.
std::string emboss_to_json(const Slic3r::ModelVolume &volume);
void emboss_from_json(Slic3r::ModelVolume &volume, const std::string &json);
// Model::plates_custom_gcodes <-> JSON (Vec<PlateCustomGcodes> côté Rust).
std::string custom_gcodes_to_json(const Slic3r::Model &model);
void custom_gcodes_from_json(Slic3r::Model &model, const std::string &json, int only_plate = -1,
                             int as_plate = -1);

void write_project_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                           rust::Str out_path, rust::Str custom_gcodes_json);

struct RawObjColors;
RawObjColors load_obj_colors_raw(rust::Str path);

struct RawCalibProject;
RawCalibProject calib_project_raw(rust::Str kind, double start, double end, double step, rust::Str config_json,
                                  rust::Str resources, rust::Str extra_json);

struct RawProjectPlate;
void write_project_plates_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                              rust::Str custom_gcodes_json,
                              const rust::Vec<RawProjectPlate> &plates, rust::Str aux_dir, rust::Str out_path);
rust::Vec<rust::String> read_project_auxiliaries_raw(rust::Str path, rust::Str out_dir);

struct RawPlateGcode;
void write_gcode_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                         const rust::Vec<RawPlateGcode> &plates, int32_t export_plate_idx,
                         rust::Str out_path);

RawRepairResult repair_mesh_raw(const RawMesh &mesh);

struct RawCalibParams;
RawSliceResult slice_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                         rust::Str work_dir, const RawCalibParams &calib,
                         rust::Str custom_gcodes_json, bool allow_mixed_temperatures);
struct RawValidateIssue;
rust::Vec<RawValidateIssue> validate_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                                         const RawCalibParams &calib, rust::Str custom_gcodes_json,
                                         bool allow_mixed_temperatures);

rust::Vec<RawObject> fill_bed_raw(const rust::Vec<RawObject> &objects, const rust::Vec<double> &bed_xy,
                                  rust::Str config_json, uint32_t object_index, double clearance_mm);

struct RawArrangeResult;
RawArrangeResult arrange_orca_raw(const rust::Vec<RawObject> &objects, const rust::Vec<double> &bed_xy,
                                  rust::Str config_json, bool force_seq_print, bool allow_rotations);

rust::Vec<RawObject> arrange_raw(const rust::Vec<RawObject> &objects,
                                 const rust::Vec<double> &bed_xy, double clearance_mm,
                                 bool allow_rotations, bool orca_cli_mode);

RawObject orient_raw(const RawObject &object);
rust::Vec<double> layer_profile_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                                    uint8_t op, float quality, const rust::Vec<double> &profile,
                                    uint32_t radius, bool keep_min);
rust::Vec<double> adjust_layer_profile_raw(const rust::Vec<RawObject> &objects,
                                           rust::Str config_json, const rust::Vec<double> &profile,
                                           double z, double delta, double band_width,
                                           uint8_t action);

size_t model_triangle_count(rust::Str path);

size_t print_config_option_count();

} // namespace webslicer
