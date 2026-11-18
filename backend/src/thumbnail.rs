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

/// Côté des vignettes de plateau du 3MF (Orca : `THUMBNAIL_SIZE_3MF` 512 × 512).
pub const PLATE_SIZE: u32 = 512;

/// Vignette PNG 512 × 512 d'un plateau pour le 3MF exporté (rendu serveur
/// quand l'interface n'a pas fourni sa capture) : instances posées sur
/// `plate`, couleur `colour` (premier filament, défaut vert d'Orca).
/// `None` si le plateau est vide.
pub fn render_plate(model: &Model, plate: u32, colour: Option<&str>) -> Option<Vec<u8>> {
    let mut only = model.clone();
    for object in &mut only.objects {
        object.instances.retain(|i| i.plate_index == plate);
    }
    only.objects.retain(|o| !o.instances.is_empty());
    let triangles = crate::thumbnails::world_triangles(&only);
    if triangles.is_empty() {
        return None;
    }
    let colour = colour
        .filter(|c| c.starts_with('#') && c.len() >= 7)
        .map_or(COLOUR, crate::thumbnails::parse_colour);
    let rgba = crate::thumbnails::render_rgba(&triangles, colour, PLATE_SIZE, PLATE_SIZE);
    Some(crate::thumbnails::encode_png(&rgba, PLATE_SIZE, PLATE_SIZE))
}

/// Contrôle d'une vignette PNG envoyée par l'interface : signature, IHDR,
/// 8 bits RGB ou RGBA non entrelacé (formats lus par `decode_colored_png` de
/// libslic3r), 16 à 4096 px de côté. Renvoie (largeur, hauteur).
pub fn check_png(bytes: &[u8]) -> Result<(u32, u32), &'static str> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 33 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err("PNG attendu");
    }
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    let (width, height) = (be(16), be(20));
    let (depth, colour_type, interlace) = (bytes[24], bytes[25], bytes[28]);
    if !(16..=4096).contains(&width) || !(16..=4096).contains(&height) {
        return Err("dimensions hors plage (16–4096 px)");
    }
    if depth != 8 || !(colour_type == 2 || colour_type == 6) || interlace != 0 {
        return Err("PNG 8 bits RGB ou RGBA non entrelacé attendu");
    }
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::TriangleMesh;

    #[test]
    fn png_check_accepts_rendered_and_rejects_others() {
        let png = crate::thumbnails::encode_png(&vec![255; 32 * 32 * 4], 32, 32);
        assert_eq!(check_png(&png), Ok((32, 32)));
        assert!(check_png(b"GIF89a....").is_err());
        let tiny = crate::thumbnails::encode_png(&[0; 4 * 4 * 4], 4, 4);
        assert!(check_png(&tiny).is_err());
    }

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
