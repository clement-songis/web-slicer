//! Déclaration cxx du bridge vers libslic3r (étendu par domaine, R1).
//!
//! Convention : les fonctions C++ jettent (std::exception) ; cxx convertit
//! en `Result` côté Rust. Les données traversent en structs partagées
//! plates (Vec de primitifs), remappées vers `engine::api` côté Rust.

#[cxx::bridge(namespace = "webslicer")]
pub(crate) mod ffi {
    /// Maillage plat : positions xyz consécutives + indices par triplets.
    #[derive(Debug, Default)]
    struct RawMesh {
        vertices: Vec<f32>,
        indices: Vec<u32>,
    }

    /// Volume d'un objet (pièce/modificateur/…) avec sa transformation.
    #[derive(Debug)]
    struct RawVolume {
        name: String,
        /// Matrice 4×4 colonne-major (Transform3d Eigen).
        matrix: [f64; 16],
        /// ModelVolumeType : 0=part, 1=negative, 2=modifier, 3=blocker, 4=enforcer.
        role: u8,
        /// Extrudeur attribué (0 = héritage).
        extruder: u32,
        mesh: RawMesh,
    }

    /// Instance d'un objet sur le plateau.
    #[derive(Debug)]
    struct RawInstance {
        matrix: [f64; 16],
    }

    /// Objet complet de la scène.
    #[derive(Debug)]
    struct RawObject {
        name: String,
        volumes: Vec<RawVolume>,
        instances: Vec<RawInstance>,
    }

    /// Projet 3MF : scène + config embarquée (clé → valeur sérialisée Orca,
    /// transportée en JSON).
    #[derive(Debug)]
    struct RawProject {
        objects: Vec<RawObject>,
        config_json: String,
    }

    /// Résultat de tranchage : chemin du G-code + statistiques minimales.
    #[derive(Debug, Default)]
    struct RawSliceResult {
        gcode_path: String,
        estimated_time_s: f64,
        /// Filament consommé par extrudeur, en mm.
        filament_mm: Vec<f64>,
        /// Poids total du filament, en g (agrégé).
        filament_g: f64,
        layer_count: u32,
        tool_changes: u32,
        /// Vignettes PNG écrites dans work_dir.
        thumbnails: Vec<String>,
    }

    /// Résultat de réparation : maillage corrigé + compteurs admesh.
    #[derive(Debug)]
    struct RawRepairResult {
        mesh: RawMesh,
        edges_fixed: i32,
        degenerate_facets: i32,
        facets_removed: i32,
        facets_reversed: i32,
        backwards_edges: i32,
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/model.hpp");

        /// Initialise l'état global de libslic3r (répertoires temp/data —
        /// sans quoi le lecteur 3MF tente d'écrire ses backups à la racine).
        fn init_runtime(temp_dir: &str, data_dir: &str);

        /// Charge un fichier modèle via `Slic3r::Model::read_from_file`
        /// (STL/OBJ/3MF géométrie, STEP via OCCT) → scène brute.
        fn load_model_raw(path: &str) -> Result<Vec<RawObject>>;

        /// Lit un projet 3MF OrcaSlicer : scène + config embarquée.
        fn read_project_3mf_raw(path: &str) -> Result<RawProject>;

        /// Écrit un projet 3MF compatible OrcaSlicer.
        fn write_project_3mf_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            out_path: &str,
        ) -> Result<()>;

        /// Arrangement automatique dans le contour de plateau (mm).
        fn arrange_raw(
            objects: &Vec<RawObject>,
            bed_xy: &Vec<f64>,
            clearance_mm: f64,
        ) -> Result<Vec<RawObject>>;

        /// Orientation automatique d'un objet.
        fn orient_raw(object: &RawObject) -> Result<RawObject>;

        /// Répare un maillage (admesh : arêtes, dégénérés, orientation).
        fn repair_mesh_raw(mesh: &RawMesh) -> Result<RawRepairResult>;

        /// Tranche un plateau via `Slic3r::Print` (apply → process →
        /// export_gcode). La progression est émise directement sur la sortie
        /// standard (`P <ratio> <phase>`) : appelé depuis le process
        /// engine-worker dont stdout est le pipe de progression (T019).
        fn slice_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            work_dir: &str,
        ) -> Result<RawSliceResult>;

        /// Nombre total de triangles d'un fichier (smoke T012).
        fn model_triangle_count(path: &str) -> Result<usize>;

        /// Nombre d'options du `print_config_def` runtime.
        fn print_config_option_count() -> usize;
    }
}
