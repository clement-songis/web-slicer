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
        /// Peintures, vecteurs parallèles : canal (0=supports, 1=couture,
        /// 2=MMU, 3=fuzzy), triangle (croissant par canal), état hex Orca.
        paint_channel: Vec<u8>,
        paint_triangle: Vec<u32>,
        paint_state: Vec<String>,
        /// Surcharges `ModelVolume.config` (clé → valeur Orca), ex. modificateur.
        config_json: String,
    }

    /// Instance d'un objet sur le plateau.
    #[derive(Debug)]
    struct RawInstance {
        matrix: [f64; 16],
        printable: bool,
    }

    /// Résultat de l'arrangement du CLI Orca : scène arrangée et
    /// `ModelInstance::arrange_order` par instance (ordre objets/instances).
    #[derive(Debug)]
    struct RawArrangeResult {
        objects: Vec<RawObject>,
        arrange_order: Vec<i32>,
        is_seq_print: bool,
    }

    /// Projet de calibration (portage des Plater::calib_* d'Orca).
    #[derive(Debug)]
    struct RawCalibProject {
        name: String,
        objects: Vec<RawObject>,
        /// Surcharges des presets édités par Orca (JSON clé → valeur Orca).
        overrides_json: String,
        calib_mode: i32,
        calib_start: f64,
        calib_end: f64,
        calib_step: f64,
        /// G-code personnalisés par couche (motif PA, comme plates_custom_gcodes).
        custom_gcodes_json: String,
    }

    /// Plateau d'un projet 3MF : index, nom, réglages du plateau (JSON clé →
    /// valeur Orca) et paires plates (objet, instance) posées dessus.
    #[derive(Debug)]
    struct RawProjectPlate {
        plate_index: u32,
        name: String,
        config_json: String,
        obj_inst: Vec<u32>,
    }

    /// Plateau d'un .gcode.3mf : G-code tranché (vide = non tranché) et
    /// paires plates (objet, instance) posées sur ce plateau.
    #[derive(Debug)]
    struct RawPlateGcode {
        plate_index: u32,
        gcode_path: String,
        obj_inst: Vec<u32>,
    }

    /// Objet complet de la scène.
    #[derive(Debug)]
    struct RawObject {
        name: String,
        volumes: Vec<RawVolume>,
        instances: Vec<RawInstance>,
        /// Surcharges `ModelObject.config` (clé → valeur Orca).
        config_json: String,
        /// Profil de hauteur variable, vecteur plat `<z, hauteur>`.
        layer_height_profile: Vec<f64>,
        /// `layer_config_ranges` : JSON `[{"min","max","config":{clé → valeur Orca}}]`.
        layer_ranges_json: String,
    }

    /// Projet 3MF : scène + config embarquée (clé → valeur sérialisée Orca,
    /// transportée en JSON).
    #[derive(Debug)]
    struct RawProject {
        objects: Vec<RawObject>,
        config_json: String,
        /// `Model::plates_custom_gcodes` en JSON (`Vec<PlateCustomGcodes>`).
        custom_gcodes_json: String,
        /// Plateaux du projet (`PlateData` lus par `load_bbs_3mf`) : index,
        /// nom, réglages du plateau, paires (objet, instance) du modèle chargé.
        plates: Vec<RawProjectPlate>,
        /// Config au format project_settings.config d'Orca (vecteurs en tableaux).
        project_config_json: String,
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
        /// `Print::output_filename(base)` d'Orca (gabarit `filename_format`
        /// évalué par le PlaceholderParser), base = OUTPUT_FILENAME_BASE_TOKEN ;
        /// vide si le gabarit n'a pas pu être évalué.
        output_filename: String,
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

    /// Résultat de simplification quadrique : maillage décimé + dernière
    /// erreur quadrique acceptée (sortie `max_error` de libslic3r).
    #[derive(Debug)]
    struct RawSimplifyResult {
        mesh: RawMesh,
        max_error: f32,
    }

    /// Plan de coupe (split.cpp) : point + normale dans le repère du maillage,
    /// moitiés à conserver (attributs KeepUpper/KeepLower de `Cut`).
    #[derive(Debug)]
    struct RawCutPlane {
        point: [f64; 3],
        normal: [f64; 3],
        keep_upper: bool,
        keep_lower: bool,
    }

    /// Résultat de coupe : moitiés (vides si non conservées) + nombre de
    /// facettes produites de chaque côté avant filtrage.
    #[derive(Debug)]
    struct RawCutResult {
        upper: RawMesh,
        lower: RawMesh,
        upper_facets: u32,
        lower_facets: u32,
    }

    /// `Calib_Params` d'Orca (sous-ensemble) : mode = valeur de `CalibMode`
    /// (0 = aucun), posé par `Print::set_calib_params` avant le tranchage.
    #[derive(Debug, Default)]
    struct RawCalibParams {
        mode: i32,
        start: f64,
        end: f64,
        step: f64,
        print_numbers: bool,
        freq_start_x: f64,
        freq_end_x: f64,
        freq_start_y: f64,
        freq_end_y: f64,
        test_model: i32,
        shaper_type: String,
    }

    /// Facettes peintes d'un canal : triangle (croissant) → chaîne hex Orca.
    #[derive(Debug, Default)]
    struct RawPaintFacets {
        triangles: Vec<u32>,
        states: Vec<String>,
    }

    /// Trait de pinceau (repère du maillage) : points xyz + facette touchée par
    /// point, position caméra (repère maillage), rayon (mm monde), curseur
    /// sphère/cercle, état (1 enforcer, 2 blocker, n extrudeur MMU), matrice
    /// monde colonne-major, subdivision, « peindre surplombs seulement » (°).
    #[derive(Debug)]
    struct RawPaintStroke {
        points: Vec<f32>,
        facets: Vec<u32>,
        camera_pos: [f32; 3],
        radius: f32,
        sphere: bool,
        state: u8,
        trafo: [f64; 16],
        splitting: bool,
        highlight_deg: f32,
        /// 0 = pinceau (`select_patch`), 1 = smart fill (`seed_fill_select_triangles`),
        /// 2 = pot de peinture (`bucket_fill_select_triangles`, propagation).
        tool: u8,
        /// Angle d'arrêt des remplissages (°, `m_smart_fill_angle`).
        fill_angle: f32,
    }

    /// Résultat de `GCodeProcessor` en colonnes (une ligne par `MoveVertex`) :
    /// `bytes` ×4 (type, rôle, extrudeur, couleur), `floats` ×16 (x, y, z,
    /// feedrate, actual_feedrate, mm3_per_mm, width, height, fan, temperature,
    /// acceleration, jerk, pressure_advance, time_normal, time_stealth, delta_e),
    /// `ints` ×2 (layer_id, gcode_id) ; `summary_json` = statistiques.
    #[derive(Debug, Default)]
    struct RawGcodeProcess {
        bytes: Vec<u8>,
        floats: Vec<f32>,
        ints: Vec<u32>,
        summary_json: String,
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/gcode_process.hpp");

        /// `GCodeProcessor::process_file` (simulation cinématique d'Orca).
        fn gcode_process_raw(path: &str) -> Result<RawGcodeProcess>;
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/flush.hpp");

        /// Volumes de purge depuis les couleurs (FlushVolCalculator d'Orca).
        fn flush_volumes_raw(
            colours: &Vec<String>,
            is_support: &Vec<bool>,
            min_flush: &Vec<i32>,
            flush_dataset: i32,
        ) -> Result<Vec<f64>>;

        /// Minimum de purge par filament (portage de get_min_flush_volumes, GUI).
        fn min_flush_volumes_raw(config_json: &str, nozzle_id: u32) -> Result<Vec<i32>>;
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/export_mesh.hpp");

        /// Maillage d'export par objet comme `Plater::export_stl` d'Orca
        /// (pièces positives, négatifs soustraits en CSG, instances fusionnées).
        fn export_meshes_raw(objects: &Vec<RawObject>) -> Result<Vec<RawMesh>>;
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/paint.hpp");

        /// Session de peinture : maillage + un `TriangleSelector` par canal.
        type PaintSession;

        fn paint_session_new(mesh: &RawMesh) -> Result<UniquePtr<PaintSession>>;
        fn paint_session_triangles(session: &PaintSession) -> usize;
        /// Remplace l'état d'un canal (chaînes 3MF → `TriangleSelector::deserialize`).
        fn paint_session_load(
            session: Pin<&mut PaintSession>,
            channel: u8,
            facets: &RawPaintFacets,
        ) -> Result<()>;
        /// Applique un trait (`select_patch`, curseurs de GLGizmoPainterBase).
        fn paint_session_stroke(
            session: Pin<&mut PaintSession>,
            channel: u8,
            stroke: &RawPaintStroke,
        ) -> Result<()>;
        /// État du canal en chaînes 3MF (`FacetsAnnotation::get_triangle_as_string`).
        fn paint_session_export(session: &PaintSession, channel: u8) -> Result<RawPaintFacets>;
    }

    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/model.hpp");
        include!("engine/src/adapters/ffi/bridge/mesh.hpp");
        include!("engine/src/adapters/ffi/bridge/split.hpp");

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
            custom_gcodes_json: &str,
        ) -> Result<()>;

        /// Écrit un projet 3MF avec ses plateaux comme « Save project » d'Orca.
        fn write_project_plates_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            custom_gcodes_json: &str,
            plates: &Vec<RawProjectPlate>,
            out_path: &str,
        ) -> Result<()>;

        /// Écrit un .gcode.3mf comme « Export (all) plate sliced file » d'Orca
        /// (`export_plate_idx` < 0 = tous les plateaux).
        fn write_gcode_3mf_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            plates: &Vec<RawPlateGcode>,
            export_plate_idx: i32,
            out_path: &str,
        ) -> Result<()>;

        /// Arrangement automatique dans le contour de plateau (mm).
        fn arrange_raw(
            objects: &Vec<RawObject>,
            bed_xy: &Vec<f64>,
            clearance_mm: f64,
            allow_rotations: bool,
            orca_cli_mode: bool,
        ) -> Result<Vec<RawObject>>;

        /// Arrangement du CLI Orca piloté par la config d'impression
        /// (séquentiel si print_sequence = by object ou forcé).
        fn arrange_orca_raw(
            objects: &Vec<RawObject>,
            bed_xy: &Vec<f64>,
            config_json: &str,
            force_seq_print: bool,
            allow_rotations: bool,
        ) -> Result<RawArrangeResult>;

        /// « Remplir le plateau avec des instances » d'Orca (FillBedJob) :
        /// renvoie la scène avec les copies ajoutées à l'objet `object_index`.
        fn fill_bed_raw(
            objects: &Vec<RawObject>,
            bed_xy: &Vec<f64>,
            config_json: &str,
            object_index: u32,
            clearance_mm: f64,
        ) -> Result<Vec<RawObject>>;

        /// Orientation automatique d'un objet.
        fn orient_raw(object: &RawObject) -> Result<RawObject>;

        /// Projet de calibration d'Orca (modèle, coupes, échelle, réglages).
        fn calib_project_raw(
            kind: &str,
            start: f64,
            end: f64,
            step: f64,
            config_json: &str,
            resources: &str,
            extra_json: &str,
        ) -> Result<RawCalibProject>;

        /// Répare un maillage (admesh : arêtes, dégénérés, orientation).
        fn repair_mesh_raw(mesh: &RawMesh) -> Result<RawRepairResult>;

        /// Simplifie un maillage via `Slic3r::its_quadric_edge_collapse`
        /// (QuadricEdgeCollapse.hpp, outil « Simplifier » d'Orca).
        /// `triangle_count` = cible (0 = aucune), `max_error` = erreur
        /// quadrique maximale (`f32::MAX` = illimitée).
        fn simplify_mesh_raw(
            mesh: &RawMesh,
            triangle_count: u32,
            max_error: f32,
        ) -> Result<RawSimplifyResult>;

        /// Écrit un maillage au format Draco via `Slic3r::store_drc`
        /// (Format/DRC.cpp) ; `bits` = quantification des positions (0 = aucune).
        fn store_drc_raw(mesh: &RawMesh, path: &str, bits: i32) -> Result<()>;

        /// Tranche un plateau via `Slic3r::Print` (apply → process →
        /// export_gcode). La progression est émise directement sur la sortie
        /// standard (`P <ratio> <phase>`) : appelé depuis le process
        /// engine-worker dont stdout est le pipe de progression (T019).
        fn slice_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            work_dir: &str,
            calib: &RawCalibParams,
            custom_gcodes_json: &str,
        ) -> Result<RawSliceResult>;

        /// Sépare un maillage en composantes connexes (`TriangleMesh::split`,
        /// pièces < 3 facettes ignorées comme `ModelObject::split`).
        fn split_mesh_raw(mesh: &RawMesh) -> Result<Vec<RawMesh>>;

        /// Coupe plane (`cut_mesh` avec bouchage, repère du plan comme `Cut`).
        fn cut_mesh_raw(mesh: &RawMesh, plane: &RawCutPlane) -> Result<RawCutResult>;

        /// Booléen mcut (`MeshBoolean::mcut::make_boolean`) : 0 = UNION,
        /// 1 = A_NOT_B, 2 = INTERSECTION.
        fn boolean_mesh_raw(a: &RawMesh, b: &RawMesh, op: u8) -> Result<RawMesh>;

        /// Subdivision de Loop (`TriangleMeshDeal::smooth_triangle_mesh`, igl::loop).
        fn smooth_mesh_raw(mesh: &RawMesh) -> Result<RawMesh>;

        /// Texte en relief (`Emboss::text2shapes` + `polygons2model`), police TTF.
        fn emboss_text_raw(
            font_path: &str,
            text: &str,
            size_mm: f32,
            depth_mm: f32,
            boldness_mm: f32,
        ) -> Result<RawMesh>;

        /// Forme SVG en relief (nanosvg → `create_shape_with_ids` → `polygons2model`).
        fn emboss_svg_raw(svg: &str, width_mm: f32, depth_mm: f32) -> Result<RawMesh>;

        /// Primitive d'Orca (`ObjectList::create_mesh`) : `its_make_*` avec les
        /// paramètres de la GUI. `shape` = `PrimitiveShape::code`.
        fn make_primitive_raw(shape: u8, side_mm: f64) -> Result<RawMesh>;

        /// Profil de hauteur variable (Slicing.cpp) de `objects[0]` : op 0 =
        /// `layer_height_profile_adaptive(quality)`, 1 = `smooth_height_profile(
        /// profile, {radius, keep_min})`. `SlicingParameters` construit par
        /// `PrintObject::slicing_parameters` depuis la config complète.
        fn layer_profile_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            op: u8,
            quality: f32,
            profile: &Vec<f64>,
            radius: u32,
            keep_min: bool,
        ) -> Result<Vec<f64>>;

        /// Pinceau de hauteur de couche : `adjust_layer_height_profile`
        /// (Slicing.cpp:518) ; action 0 = increase, 1 = decrease, 2 = reduce,
        /// 3 = smooth (`LayerHeightEditActionType`).
        fn adjust_layer_profile_raw(
            objects: &Vec<RawObject>,
            config_json: &str,
            profile: &Vec<f64>,
            z: f64,
            delta: f64,
            band_width: f64,
            action: u8,
        ) -> Result<Vec<f64>>;

        /// Nombre total de triangles d'un fichier (smoke T012).
        fn model_triangle_count(path: &str) -> Result<usize>;

        /// Nombre d'options du `print_config_def` runtime.
        fn print_config_option_count() -> usize;
    }
}
