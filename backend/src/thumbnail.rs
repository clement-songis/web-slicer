//! Vignette de projet de la bibliothèque (F21) : Orca enregistre une vignette
//! de la scène à chaque sauvegarde (Metadata/plate_1.png, GLCanvas3D). Rendue
//! ici par le rasteriseur logiciel des vignettes G-code (`thumbnails`), en
//! PNG 256×256 fond transparent, couleur par défaut d'Orca.

use engine::api::Model;

pub const SIZE: u32 = 256;
/// Vert des vignettes d'Orca (filament par défaut #00AE42).
const COLOUR: [f64; 3] = [0.0, 174.0 / 255.0, 66.0 / 255.0];

/// PNG de la scène, `None` si elle est vide.
pub fn render(model: &Model) -> Option<Vec<u8>> {
    let triangles = crate::thumbnails::world_triangles(model);
    if triangles.is_empty() {
        return None;
    }
    let rgba = crate::thumbnails::render_rgba(&triangles, COLOUR, SIZE, SIZE);
    Some(crate::thumbnails::encode_png(&rgba, SIZE, SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::TriangleMesh;

    #[test]
    fn cube_gives_png_and_empty_scene_none() {
        assert!(render(&Model::default()).is_none());
        let s = 10.0;
        let mut model = Model::default();
        model.add_object(
            "cube",
            TriangleMesh {
                vertices: vec![
                    [0.0, 0.0, 0.0],
                    [s, 0.0, 0.0],
                    [s, s, 0.0],
                    [0.0, s, 0.0],
                    [0.0, 0.0, s],
                    [s, 0.0, s],
                    [s, s, s],
                    [0.0, s, s],
                ],
                indices: vec![
                    [0, 2, 1],
                    [0, 3, 2],
                    [4, 5, 6],
                    [4, 6, 7],
                    [0, 1, 5],
                    [0, 5, 4],
                    [1, 2, 6],
                    [1, 6, 5],
                    [2, 3, 7],
                    [2, 7, 6],
                    [3, 0, 4],
                    [3, 4, 7],
                ],
            },
        );
        let png = render(&model).expect("png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() > 200);
    }
}
