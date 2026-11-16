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
