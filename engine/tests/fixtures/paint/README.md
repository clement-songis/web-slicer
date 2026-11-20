# Référence de peinture Orca (TriangleSelector)

`circle.txt` / `sphere.txt` : chaînes 3MF (`slic3rpe:custom_supports`) produites
par **libslic3r seul**, sans le bridge web-slicer, par `paint_ref.cpp` :
cube 20 mm, trait de 3 points sur la face supérieure ((5,5) → (12,8) → (8,14),
facettes 2,2,3), rayon 3 mm, caméra (10,10,100), enforcer, subdivision active —
mêmes appels que `GLGizmoPainterBase` (DoublePointCursor + `select_patch`) puis
`FacetsAnnotation::set` / `get_triangle_as_string`.

Régénérer (devShell) : `nix develop -c engine/tests/fixtures/paint/build.sh`.
Format : `<triangle> <hex>` par ligne. Comparé par `engine/tests/ffi_paint.rs`.

Remplissages (`smart*.txt`, `bucket*.txt`) : cylindre `its_make_cylinder(10, 20)`
(180 secteurs), clic sur la paroi (facette et point en tête de fichier, ligne `#`),
`seed_fill_select_triangles` / `bucket_fill_select_triangles` puis
`seed_fill_apply_on_triangles(ENFORCER)` (GLGizmoPainterBase.cpp:851-866) ; angle
d'arrêt 30° (défaut `m_smart_fill_angle`) → paroi entière (360 triangles), 1° → 2.

Lot 38 (outils du gizmo) — mêmes conventions, une ligne `#` d'en-tête si un
point/facette est calculé par la référence :
- `pointer.txt` : trait cercle du cube (enforcer) puis curseur « Triangles »
  (CursorType::POINTER → `bucket_fill_select_triangles(hit, facet, clp, -1, false)`)
  en (6, 4.5, 20), état BLOCKER : seul le sous-triangle touché change.
- `height.txt` : cylindre, sélecteur MMU (edge_limit 0,2), plage de hauteur 2 mm
  (CursorType::HEIGHT_RANGE, `SinglePointCursor::cursor_factory(z_world, …)`),
  clic paroi, état Extruder2 ; `height_z.txt` : autre pièce (trafo z +3), cote
  monde imposée 8, première facette dans la plage (get_projected_height_range).
- `gap.txt` / `gap0.txt` : cube MMU, trait Extruder1 + tampon Extruder2 de
  rayon 0,8 ; comblement à 4 mm² / 0 mm². Le comblement est du code GUI
  (TriangleSelectorPatch) : la référence en contient une copie verbatim
  (GLGizmoPainterBase.cpp:1423-1530) dans `GapRef`.
- `remap_input.txt` / `remap.txt` : cube MMU, trait Extruder1 + tampon
  Extruder2 (rayon 2), avant / après `remap_triangle_state` (1 → 3, 2 → 1).
- `bucket_noedge.txt` : pot sans détection d'arêtes (angle -1) depuis la paroi
  du cylindre, Extruder3 → tout le cylindre (720 triangles).
