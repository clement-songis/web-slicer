// Formes primitives du menu « Ajouter » (cube, cylindre, sphère, cône, disque,
// tore) : le maillage est créé côté serveur par libslic3r
// (POST /api/projects/{id}/primitive, its_make_* avec les paramètres d'Orca,
// GUI_ObjectList::create_mesh). L'ancien générateur TS a été retiré ; seul le
// type de forme reste ici.
export type PrimitiveKind = 'cube' | 'cylinder' | 'sphere' | 'cone' | 'disc' | 'torus';
