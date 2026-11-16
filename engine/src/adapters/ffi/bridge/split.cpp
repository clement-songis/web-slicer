// Wrappers de découpe : mêmes primitives libslic3r que la GUI d'Orca.
// - split   : TriangleMesh::split (ModelObject::split / ModelVolume::split),
//             pièces < 3 facettes ignorées comme dans ModelObject::split ;
// - cut     : plan ramené sur z = 0 puis cut_mesh avec bouchage des sections
//             (process_volume_cut de CutUtils.cpp) ;
// - boolean : MeshBoolean::mcut::make_boolean (GLGizmoMeshBoolean :
//             "UNION", "A_NOT_B", "INTERSECTION").
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge/split.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/ClipperUtils.hpp"
#include "libslic3r/Emboss.hpp"
#include "libslic3r/NSVGUtils.hpp"
#include "libslic3r/MeshBoolean.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/TriangleMeshDeal.hpp"
#include "libslic3r/TriangleMeshSlicer.hpp"

#include <Eigen/Geometry>

#include <climits>
#include <cmath>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace webslicer {

namespace {

// Contrôles d'entrée (libslic3r n'en fait aucun) puis conversion en its.
indexed_triangle_set raw_to_its(const RawMesh &raw)
{
    if (raw.indices.empty() || raw.indices.size() % 3 != 0 ||
        raw.vertices.empty() || raw.vertices.size() % 3 != 0)
        throw std::runtime_error("maillage vide ou indices incomplets");
    const size_t vertex_count = raw.vertices.size() / 3;
    if (vertex_count > static_cast<size_t>(INT32_MAX))
        throw std::runtime_error("trop de sommets pour un indexed_triangle_set");
    for (size_t i = 0; i < raw.indices.size(); ++i)
        if (raw.indices[i] >= vertex_count)
            throw std::runtime_error("indice de sommet hors limites");
    for (size_t i = 0; i < raw.vertices.size(); ++i)
        if (!std::isfinite(raw.vertices[i]))
            throw std::runtime_error("coordonnée de sommet non finie");

    indexed_triangle_set its;
    its.vertices.reserve(vertex_count);
    for (size_t i = 0; i < raw.vertices.size(); i += 3)
        its.vertices.emplace_back(raw.vertices[i], raw.vertices[i + 1], raw.vertices[i + 2]);
    its.indices.reserve(raw.indices.size() / 3);
    for (size_t i = 0; i < raw.indices.size(); i += 3)
        its.indices.emplace_back(static_cast<int>(raw.indices[i]),
                                 static_cast<int>(raw.indices[i + 1]),
                                 static_cast<int>(raw.indices[i + 2]));
    return its;
}

RawMesh its_to_raw_mesh(const indexed_triangle_set &its)
{
    RawMesh out;
    out.vertices.reserve(its.vertices.size() * 3);
    for (const auto &v : its.vertices) {
        out.vertices.push_back(v.x());
        out.vertices.push_back(v.y());
        out.vertices.push_back(v.z());
    }
    out.indices.reserve(its.indices.size() * 3);
    for (const auto &f : its.indices) {
        out.indices.push_back(static_cast<uint32_t>(f.x()));
        out.indices.push_back(static_cast<uint32_t>(f.y()));
        out.indices.push_back(static_cast<uint32_t>(f.z()));
    }
    return out;
}

} // namespace

rust::Vec<RawMesh> split_mesh_raw(const RawMesh &raw)
{
    Slic3r::TriangleMesh mesh(raw_to_its(raw));
    // TriangleMesh::split : its_split + retournement des pièces de volume
    // négatif (« Some source mesh parts may be incorrectly oriented »).
    std::vector<Slic3r::TriangleMesh> parts = mesh.split();
    rust::Vec<RawMesh> out;
    for (Slic3r::TriangleMesh &part : parts) {
        // ModelObject::split : « if (mesh.facets_count() < 3) continue; »
        if (part.facets_count() < 3)
            continue;
        out.push_back(its_to_raw_mesh(part.its));
    }
    return out;
}

RawCutResult cut_mesh_raw(const RawMesh &raw, const RawCutPlane &plane)
{
    const Eigen::Vector3d point(plane.point[0], plane.point[1], plane.point[2]);
    Eigen::Vector3d normal(plane.normal[0], plane.normal[1], plane.normal[2]);
    if (!point.allFinite() || !normal.allFinite() || normal.norm() < 1e-9)
        throw std::runtime_error("plan de coupe invalide");
    normal.normalize();

    // Repère de coupe (comme Cut::perform_with_plane : inverse de la matrice du
    // plan, rotation puis translation) : le plan devient z = 0, normale +Z.
    Slic3r::Transform3d to_plane = Slic3r::Transform3d::Identity();
    to_plane.rotate(Eigen::Quaterniond::FromTwoVectors(normal, Eigen::Vector3d::UnitZ()));
    to_plane.translate(-point);
    const Slic3r::Transform3d from_plane = to_plane.inverse();

    Slic3r::TriangleMesh mesh(raw_to_its(raw));
    mesh.transform(to_plane, /*fix_left_handed=*/true);

    indexed_triangle_set upper_its, lower_its;
    Slic3r::cut_mesh(mesh.its, 0.0f, &upper_its, &lower_its, /*triangulate_caps=*/true);

    auto finish = [&](indexed_triangle_set &its, bool keep) {
        // cut_mesh recopie tous les sommets de l'entrée dans chaque moitié :
        // compaction pour ne garder que ceux référencés.
        Slic3r::its_compactify_vertices(its, /*shrink_to_fit=*/true);
        if (!keep || its.indices.empty())
            return RawMesh{};
        Slic3r::TriangleMesh half(std::move(its));
        half.transform(from_plane, /*fix_left_handed=*/true);
        return its_to_raw_mesh(half.its);
    };

    RawCutResult result;
    result.upper_facets = static_cast<uint32_t>(upper_its.indices.size());
    result.lower_facets = static_cast<uint32_t>(lower_its.indices.size());
    result.upper = finish(upper_its, plane.keep_upper);
    result.lower = finish(lower_its, plane.keep_lower);
    return result;
}

RawMesh boolean_mesh_raw(const RawMesh &a, const RawMesh &b, uint8_t op)
{
    const char *opts = nullptr;
    switch (op) {
    case 0: opts = "UNION"; break;
    case 1: opts = "A_NOT_B"; break;
    case 2: opts = "INTERSECTION"; break;
    default: throw std::runtime_error("opération booléenne inconnue");
    }
    Slic3r::TriangleMesh src(raw_to_its(a));
    Slic3r::TriangleMesh tool(raw_to_its(b));
    std::vector<Slic3r::TriangleMesh> results;
    Slic3r::MeshBoolean::mcut::make_boolean(src, tool, results, opts);
    // GLGizmoMeshBoolean : résultat vide = « Unable to perform boolean
    // operation on selected parts ».
    if (results.empty() || results.front().empty())
        throw std::runtime_error("opération booléenne impossible sur ces maillages "
                                 "(résultat vide)");
    // mcut peut laisser des sommets non référencés (ex. A_NOT_B) : compaction
    // pour une boîte englobante et un WSMh exacts.
    indexed_triangle_set its = std::move(results.front().its);
    Slic3r::its_compactify_vertices(its, /*shrink_to_fit=*/true);
    return its_to_raw_mesh(its);
}

// Subdivision de Loop (ObjectList::smooth_mesh d'Orca) :
// TriangleMeshDeal::smooth_triangle_mesh (igl::loop), chaque triangle → 4.
// Plater::can_smooth_mesh désactive l'entrée si le maillage a des arêtes
// ouvertes (igl::loop exige une surface manifold) : refusé ici de même.
RawMesh smooth_mesh_raw(const RawMesh &raw)
{
    Slic3r::TriangleMesh mesh(raw_to_its(raw));
    if (mesh.stats().open_edges > 0 || Slic3r::its_num_open_edges(mesh.its) > 0)
        throw std::runtime_error("le maillage contient des erreurs (arêtes ouvertes) : "
                                 "réparez-le d'abord");
    bool ok = false;
    Slic3r::TriangleMesh result = Slic3r::TriangleMeshDeal::smooth_triangle_mesh(mesh, ok);
    if (!ok || result.empty())
        throw std::runtime_error("subdivision impossible sur ce maillage");
    return its_to_raw_mesh(result.its);
}

// Texte en relief (GLGizmoEmboss / EmbossJob::try_create_mesh d'Orca) :
// text2shapes (stb_truetype) → ExPolygons à l'échelle SHAPE_SCALE, puis
// polygons2model avec ProjectZ (profondeur) dans un ProjectScale ramenant
// les points en millimètres. Face avant à z = 0, extrusion vers −z.
RawMesh emboss_text_raw(rust::Str font_path, rust::Str text, float size_mm, float depth_mm,
                        float boldness_mm)
{
    if (!(size_mm > 0.f) || !(depth_mm > 0.f))
        throw std::runtime_error("taille et profondeur doivent être positives");
    const std::string path(font_path);
    const std::string utf8(text);
    if (utf8.empty())
        throw std::runtime_error("texte vide");
    auto font = Slic3r::Emboss::create_font_file(path.c_str());
    if (!font)
        throw std::runtime_error("police illisible : " + path);
    Slic3r::Emboss::FontFileWithCache cache(std::move(font));
    Slic3r::FontProp prop(size_mm);
    prop.size_in_mm = size_mm;
    // Graisse (FontProp::boldness, en mm) : décalage des contours de glyphes,
    // le réglage « Boldness » du gizmo Emboss d'Orca.
    if (boldness_mm != 0.f)
        prop.boldness = boldness_mm;
    Slic3r::ExPolygons shapes = Slic3r::Emboss::text2shapes(cache, utf8.c_str(), prop, [] { return false; });
    if (shapes.empty())
        throw std::runtime_error("aucun glyphe produit (caractères absents de la police ?)");
    const double scale = Slic3r::Emboss::get_text_shape_scale(prop, *cache.font_file);
    auto project_z = std::make_unique<Slic3r::Emboss::ProjectZ>(depth_mm / scale);
    Slic3r::Emboss::ProjectScale project(std::move(project_z), scale);
    indexed_triangle_set its = Slic3r::Emboss::polygons2model(shapes, project);
    if (its.indices.empty())
        throw std::runtime_error("maillage de texte vide");
    Slic3r::its_compactify_vertices(its, /*shrink_to_fit=*/true);
    return its_to_raw_mesh(its);
}

// Forme SVG (GLGizmoSVG d'Orca) : nanosvg → create_shape_with_ids (remplis +
// contours) → polygons2model. Mise à l'échelle pour une largeur demandée (mm),
// profondeur d'extrusion en mm ; face avant à z = 0.
RawMesh emboss_svg_raw(rust::Str svg, float width_mm, float depth_mm)
{
    if (!(width_mm > 0.f) || !(depth_mm > 0.f))
        throw std::runtime_error("largeur et profondeur doivent être positives");
    const std::string data(svg);
    Slic3r::NSVGimage_ptr image = Slic3r::nsvgParse(data, "mm");
    if (!image || image->shapes == nullptr)
        throw std::runtime_error("SVG illisible ou sans forme");
    // Tolérance en unités de l'image (mm) : 0,1 mm comme les réglages par défaut.
    Slic3r::NSVGLineParams params(0.1);
    Slic3r::ExPolygons shapes;
    for (const Slic3r::ExPolygonsWithId &s : Slic3r::create_shape_with_ids(*image, params))
        Slic3r::append(shapes, s.expoly);
    // create_shape_with_ids conserve déjà les trous dans chaque ExPolygon.
    // Ne pas appliquer union_ex ici : son remplissage non-zero reboucherait les
    // trous evenodd (comportement également évité par GLGizmoSVG).
    if (shapes.empty())
        throw std::runtime_error("SVG sans surface remplie ni contour exploitable");
    const Slic3r::BoundingBox bb = Slic3r::get_extents(shapes);
    const double width_pt = static_cast<double>(bb.size().x());
    if (!(width_pt > 0.))
        throw std::runtime_error("SVG de largeur nulle");
    // Points en unités entières ; scale ramène en mm à la largeur demandée.
    const double scale = static_cast<double>(width_mm) / width_pt;
    auto project_z = std::make_unique<Slic3r::Emboss::ProjectZ>(depth_mm / scale);
    Slic3r::Emboss::ProjectScale project(std::move(project_z), scale);
    indexed_triangle_set its = Slic3r::Emboss::polygons2model(shapes, project);
    if (its.indices.empty())
        throw std::runtime_error("maillage SVG vide");
    Slic3r::its_compactify_vertices(its, /*shrink_to_fit=*/true);
    return its_to_raw_mesh(its);
}

// Primitives de l'ajout de forme d'Orca : copie conforme de
// `create_mesh` (src/slic3r/GUI/GUI_ObjectList.cpp:2413, GUI-only car le côté
// vient du canevas — ici `side_mm` = 0,1 × plus grande dimension du plateau,
// `GLCanvas3D::get_size_proportional_to_max_bed_size(0.1)`). La tessellation
// est celle de libslic3r (`its_make_*`, TriangleMesh.cpp) avec les pas
// angulaires de la GUI. « Slab » (dépend de la boîte d'instance) n'est pas exposé.
RawMesh make_primitive_raw(uint8_t shape, double side_mm)
{
    if (!std::isfinite(side_mm) || side_mm <= 0.)
        throw std::runtime_error("côté de primitive invalide");
    const double side = side_mm;
    indexed_triangle_set its;
    switch (shape) {
    case 0: its = Slic3r::its_make_cube(side, side, side); break;
    case 1: its = Slic3r::its_make_cylinder(0.5 * side, side); break;
    case 2: its = Slic3r::its_make_sphere(0.5 * side, PI / 90); break;
    case 3: its = Slic3r::its_make_cone(0.5 * side, side); break;
    case 4: its = Slic3r::its_make_cylinder(0.5 * side, 0.2f); break;
    case 5: its = Slic3r::its_make_torus(0.5 * side, 0.125 * side, (PI / 60)); break;
    default: throw std::runtime_error("primitive inconnue");
    }
    if (its.indices.empty())
        throw std::runtime_error("primitive vide");
    return its_to_raw_mesh(its);
}

} // namespace webslicer
