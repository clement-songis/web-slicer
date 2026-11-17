// Import OBJ avec couleurs comme Orca : load_obj lit les couleurs de sommets
// (« v x y z r g b ») ou de faces (matériaux MTL, Kd) ; regroupement par
// obj_color_deal_algo (k-means libslic3r), couleurs regroupées → filaments
// 1..k (palette), puis Model::obj_import_vertex_color_deal /
// obj_import_face_color_deal (peinture MMU par facette). Portage du chemin du
// CLI (OrcaSlicer.cpp construct_assemble_list + convert_obj_cluster_colors,
// palette initialement vide) — même résultat que le réglage par défaut du
// dialogue ObjColorDialog (nouveaux filaments pour chaque couleur regroupée).
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

// OBJ.hpp n'inclut pas ses dépendances : Model.hpp (Point, RGBA) d'abord.
#include <functional>
#include <map>

#include "libslic3r/Model.hpp"
#include "libslic3r/Format/OBJ.hpp"
#include "libslic3r/TriangleSelector.hpp"

#include <algorithm>
#include <cstdio>
#include <stdexcept>

// ObjColorUtils.hpp (en-tête OpenCV) : seule la déclaration est nécessaire,
// la définition est dans liblibslic3r.a.
bool obj_color_deal_algo(std::vector<Slic3r::RGBA> &input_colors, std::vector<Slic3r::RGBA> &cluster_colors_from_algo,
                         std::vector<int> &cluster_labels_from_algo, char &cluster_number, int max_cluster);

namespace webslicer {

rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);

RawObjColors load_obj_colors_raw(rust::Str path_str)
{
    using namespace Slic3r;
    const std::string path(path_str);
    TriangleMesh mesh;
    ObjInfo obj_info;
    std::string message;
    if (!load_obj(path.c_str(), &mesh, obj_info, message))
        throw std::runtime_error("OBJ illisible : " + message);
    if (!obj_info.lost_material_name.empty())
        throw std::runtime_error("matériau introuvable dans le MTL : " + obj_info.lost_material_name);
    if (!obj_info.face_colors.empty() && obj_info.face_colors.size() < mesh.facets_count())
        throw std::runtime_error("certaines faces n'ont pas de couleur (MTL incomplet)");

    std::string name = path.substr(path.find_last_of("/\\") + 1);
    if (name.size() > 4)
        name.erase(name.size() - 4);
    Model model;
    ModelObject *object = model.add_object((name + "_1").c_str(), path.c_str(), std::move(mesh));
    object->add_instance();

    std::vector<RGBA> palette;
    const int max_filament_count = int(EnforcerBlockerType::ExtruderMax);
    auto cluster = [&](std::vector<RGBA> &input, std::vector<unsigned char> &ids) {
        std::vector<RGBA> cluster_colors;
        std::vector<int> labels;
        char cluster_number = -1;
        obj_color_deal_algo(input, cluster_colors, labels, cluster_number, max_filament_count);
        std::vector<int> maps(cluster_colors.size(), 1);
        for (size_t i = 0; i < cluster_colors.size() && int(palette.size()) < max_filament_count; i++) {
            palette.push_back(cluster_colors[i]);
            maps[i] = int(palette.size());
        }
        ids.resize(input.size());
        for (size_t i = 0; i < input.size(); i++)
            ids[i] = static_cast<unsigned char>(maps[labels[i]]);
    };
    bool colored = false;
    std::vector<unsigned char> ids;
    if (!obj_info.vertex_colors.empty()) {
        cluster(obj_info.vertex_colors, ids);
        if (!ids.empty() && !Model::obj_import_vertex_color_deal(ids, ids.front(), &model))
            throw std::runtime_error("couleurs de sommets inapplicables");
        colored = true;
    } else if (!obj_info.face_colors.empty() && !obj_info.has_uv_png) {
        cluster(obj_info.face_colors, ids);
        if (!ids.empty() && !Model::obj_import_face_color_deal(ids, ids.front(), &model))
            throw std::runtime_error("couleurs de faces inapplicables");
        colored = true;
    }

    RawObjColors out;
    out.objects = model_to_raw(model);
    out.colored = colored;
    for (const RGBA &c : palette) {
        // Format du CLI (filament_colour « #RRGGBBAA », majuscules).
        char buf[16];
        auto ch = [](float v) { return std::clamp(int(v * 255.f), 0, 255); };
        std::snprintf(buf, sizeof(buf), "#%02X%02X%02X%02X", ch(c[0]), ch(c[1]), ch(c[2]), ch(c[3]));
        out.palette.push_back(rust::String(buf));
    }
    return out;
}

} // namespace webslicer
