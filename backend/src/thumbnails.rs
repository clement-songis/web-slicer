//! Vignettes G-code de l'export GUI d'OrcaSlicer (`GCodeThumbnails::export_thumbnails_to_file`,
//! appelé par `GCode::_do_export` avec le `thumbnail_cb` de la GUI ; le CLI n'en produit pas).
//!
//! Le rendu OpenGL d'Orca est remplacé par un rasteriseur logiciel (vue iso,
//! ombrage de Lambert, couleur du filament, fond transparent) : seules la
//! présence, les dimensions, le format et l'encodage des blocs font contrat.
//! Les blocs sont insérés là où Orca les écrit, juste après `; HEADER_BLOCK_END`,
//! avec la même syntaxe (`; <tag> begin WxH LEN`, base64 en lignes de 78).
//! Formats encodés : PNG, JPG, QOI, BTT_TFT et ColPic (port fidèle de
//! `ColPic_EncodeStr`, vérifié octet à octet contre le C++ d'Orca).

use base64::Engine as _;
use engine::api::{ConfigValue, DynamicPrintConfig, Model, VolumeRole};
use glam::{DMat4, DVec3};

/// Format d'une vignette (`GCodeThumbnailsFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailFormat {
    Png,
    Jpg,
    Qoi,
    BttTft,
    ColPic,
}

impl ThumbnailFormat {
    fn parse(ext: &str) -> Option<Self> {
        match ext.trim().to_ascii_uppercase().as_str() {
            "PNG" => Some(Self::Png),
            "JPG" => Some(Self::Jpg),
            "QOI" => Some(Self::Qoi),
            "BTT_TFT" => Some(Self::BttTft),
            "COLPIC" => Some(Self::ColPic),
            _ => None,
        }
    }
}

/// `make_and_check_thumbnail_list` + `handle_legacy_composite` : « WxH[/EXT] »
/// séparés par des virgules ; extension absente → `thumbnails_format` (puis PNG).
/// Les entrées invalides sont ignorées (Orca refuse alors la config en amont).
pub fn thumbnail_list(thumbnails: &str, default_ext: &str) -> Vec<(ThumbnailFormat, u32, u32)> {
    let mut out = Vec::new();
    for item in thumbnails.split(',') {
        let item = item.trim();
        let Some((w, rest)) = item.split_once('x') else {
            continue;
        };
        let (h, ext) = rest.split_once('/').unwrap_or((rest, ""));
        let (Ok(w), Ok(h)) = (w.trim().parse::<f64>(), h.trim().parse::<f64>()) else {
            continue;
        };
        if !(w > 0.0 && w < 1000.0 && h > 0.0 && h < 1000.0) {
            continue;
        }
        let ext = if ext.trim().is_empty() {
            if default_ext.trim().is_empty() {
                "PNG"
            } else {
                default_ext
            }
        } else {
            ext
        };
        let format = ThumbnailFormat::parse(ext).unwrap_or(ThumbnailFormat::Png);
        out.push((format, w as u32, h as u32));
    }
    out
}

fn config_text(config: &DynamicPrintConfig, key: &str) -> String {
    match config.0.get(key) {
        Some(ConfigValue::String(s)) => s.clone(),
        Some(ConfigValue::Strings(v)) => v.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

fn parse_colour(hex: &str) -> [f64; 3] {
    let hex = hex.trim().trim_start_matches('#');
    let channel = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .map_or(0.5, |v| f64::from(v) / 255.0)
    };
    if hex.len() >= 6 {
        [channel(0), channel(2), channel(4)]
    } else {
        // Défaut Orca de `filament_colour`.
        [
            0xF2 as f64 / 255.0,
            0x75 as f64 / 255.0,
            0x4E as f64 / 255.0,
        ]
    }
}

/// Triangles du plateau en repère monde (pièces de chaque instance).
pub(crate) fn world_triangles(model: &Model) -> Vec<[DVec3; 3]> {
    let mut out = Vec::new();
    for object in &model.objects {
        for instance in &object.instances {
            for volume in object
                .volumes
                .iter()
                .filter(|v| v.role == VolumeRole::ModelPart)
            {
                let m: DMat4 = instance.matrix * volume.matrix;
                let p = |i: u32| {
                    let v = volume.mesh.vertices[i as usize];
                    m.transform_point3(DVec3::new(
                        f64::from(v[0]),
                        f64::from(v[1]),
                        f64::from(v[2]),
                    ))
                };
                for &[a, b, c] in &volume.mesh.indices {
                    out.push([p(a), p(b), p(c)]);
                }
            }
        }
    }
    out
}

/// Rendu RGBA (rangées de haut en bas) d'une vue iso cadrée sur les objets.
pub fn render_rgba(triangles: &[[DVec3; 3]], colour: [f64; 3], width: u32, height: u32) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    if triangles.is_empty() || w == 0 || h == 0 {
        return rgba;
    }
    // Caméra : azimut 45° (avant-droit), élévation ~35° (vue iso d'Orca).
    let view =
        DMat4::from_rotation_x(-55f64.to_radians()) * DMat4::from_rotation_z(45f64.to_radians());
    let cam: Vec<[DVec3; 3]> = triangles
        .iter()
        .map(|t| {
            [
                view.transform_point3(t[0]),
                view.transform_point3(t[1]),
                view.transform_point3(t[2]),
            ]
        })
        .collect();
    let (mut min, mut max) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for t in &cam {
        for p in t {
            min = min.min(*p);
            max = max.max(*p);
        }
    }
    let extent = (max - min).max(DVec3::splat(1e-6));
    let scale = 0.9 * (w as f64 / extent.x).min(h as f64 / extent.y);
    let centre = (min + max) * 0.5;
    let to_screen = |p: DVec3| {
        DVec3::new(
            (p.x - centre.x) * scale + w as f64 / 2.0,
            (centre.y - p.y) * scale + h as f64 / 2.0,
            p.z,
        )
    };
    let mut depth = vec![f64::MIN; w * h];
    let light = DVec3::new(0.3, 0.4, 1.0).normalize();
    for t in &cam {
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.length_squared() == 0.0 {
            continue;
        }
        let shade = 0.3 + 0.7 * n.normalize().dot(light).abs();
        let s = [to_screen(t[0]), to_screen(t[1]), to_screen(t[2])];
        let area = (s[1].x - s[0].x) * (s[2].y - s[0].y) - (s[2].x - s[0].x) * (s[1].y - s[0].y);
        if area.abs() < 1e-12 {
            continue;
        }
        let x0 = s
            .iter()
            .map(|p| p.x)
            .fold(f64::MAX, f64::min)
            .floor()
            .max(0.0) as usize;
        let x1 = (s.iter().map(|p| p.x).fold(f64::MIN, f64::max).ceil() as usize).min(w - 1);
        let y0 = s
            .iter()
            .map(|p| p.y)
            .fold(f64::MAX, f64::min)
            .floor()
            .max(0.0) as usize;
        let y1 = (s.iter().map(|p| p.y).fold(f64::MIN, f64::max).ceil() as usize).min(h - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let w0 = ((s[1].x - px) * (s[2].y - py) - (s[2].x - px) * (s[1].y - py)) / area;
                let w1 = ((s[2].x - px) * (s[0].y - py) - (s[0].x - px) * (s[2].y - py)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * s[0].z + w1 * s[1].z + w2 * s[2].z;
                let i = y * w + x;
                if z <= depth[i] {
                    continue;
                }
                depth[i] = z;
                for c in 0..3 {
                    rgba[i * 4 + c] = (colour[c] * shade * 255.0).round().clamp(0.0, 255.0) as u8;
                }
                rgba[i * 4 + 3] = 255;
            }
        }
    }
    rgba
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// PNG RGBA 8 bits, flux zlib en blocs « stored » (valide, non compressé).
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for row in rgba.chunks(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        zlib.push(u8::from(i + 1 == chunks.len()));
        let len = chunk.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &zlib);
    chunk(b"IEND", &[]);
    out
}

/// QOI RGBA (spécification qoiformat.org) : opérations RGBA et RUN seulement.
pub fn encode_qoi(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = b"qoif".to_vec();
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[4, 0]);
    let mut prev = [0u8, 0, 0, 255];
    let mut run = 0u8;
    for px in rgba.chunks(4) {
        let px = [px[0], px[1], px[2], px[3]];
        if px == prev {
            run += 1;
            if run == 62 {
                out.push(0xC0 | (run - 1));
                run = 0;
            }
            continue;
        }
        if run > 0 {
            out.push(0xC0 | (run - 1));
            run = 0;
        }
        out.push(0xFF);
        out.extend_from_slice(&px);
        prev = px;
    }
    if run > 0 {
        out.push(0xC0 | (run - 1));
    }
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    out
}

/// JPEG baseline (JFIF, YCbCr 4:4:4, qualité 85 comme `jpeg_set_quality(&info, 85)` d'Orca).
/// Tables de quantification de l'Annexe K mises à l'échelle comme libjpeg
/// (`jpeg_quality_scaling`) ; tables de Huffman canoniques à longueur fixe
/// (DC : 12 symboles sur 4 bits, AC : 162 symboles sur 8 bits) — flux valide
/// pour tout décodeur, moins compact que les tables standard. L'alpha est
/// ignoré comme avec `JCS_EXT_RGBA`.
pub fn encode_jpg(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    const NATURAL: [usize; 64] = [
        0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27,
        20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51,
        58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
    ];
    const LUM: [u16; 64] = [
        16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69,
        56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81,
        104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
    ];
    const CHR: [u16; 64] = [
        17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99,
        99, 47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    ];
    // jpeg_quality_scaling(85) = 200 - 2*85 = 30 %.
    let scale = |t: &[u16; 64]| -> [u16; 64] {
        let mut q = [0u16; 64];
        for (o, &v) in q.iter_mut().zip(t) {
            *o = ((u32::from(v) * 30 + 50) / 100).clamp(1, 255) as u16;
        }
        q
    };
    let (qy, qc) = (scale(&LUM), scale(&CHR));
    // Symboles AC : EOB, ZRL, puis (run<<4)|size, run 0..15, size 1..10.
    let mut ac_symbols: Vec<u8> = vec![0x00, 0xF0];
    for run in 0..16u8 {
        for size in 1..=10u8 {
            ac_symbols.push((run << 4) | size);
        }
    }
    let mut ac_code = [0u16; 256];
    for (i, &s) in ac_symbols.iter().enumerate() {
        ac_code[s as usize] = i as u16; // longueur 8, code canonique = rang
    }

    let mut out = vec![0xFF, 0xD8];
    // APP0 JFIF.
    out.extend_from_slice(&[
        0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
    ]);
    for (id, q) in [(0u8, &qy), (1u8, &qc)] {
        out.extend_from_slice(&[0xFF, 0xDB, 0, 67, id]);
        out.extend(NATURAL.iter().map(|&n| q[n] as u8));
    }
    out.extend_from_slice(&[0xFF, 0xC0, 0, 17, 8]);
    out.extend_from_slice(&(height as u16).to_be_bytes());
    out.extend_from_slice(&(width as u16).to_be_bytes());
    out.extend_from_slice(&[3, 1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);
    // DHT : DC (classe 0) et AC (classe 1), même table pour luminance/chrominance.
    let mut dc_bits = [0u8; 16];
    dc_bits[3] = 12;
    let mut ac_bits = [0u8; 16];
    ac_bits[7] = ac_symbols.len() as u8;
    for (class, bits, symbols) in [
        (0x00u8, dc_bits, (0..12u8).collect::<Vec<_>>()),
        (0x10, ac_bits, ac_symbols.clone()),
    ] {
        out.extend_from_slice(&[0xFF, 0xC4]);
        out.extend_from_slice(&((19 + symbols.len()) as u16).to_be_bytes());
        out.push(class);
        out.extend_from_slice(&bits);
        out.extend_from_slice(&symbols);
    }
    out.extend_from_slice(&[0xFF, 0xDA, 0, 12, 3, 1, 0x00, 2, 0x00, 3, 0x00, 0, 63, 0]);

    let mut writer = BitWriter::default();
    let mut prev_dc = [0i32; 3];
    let (w, h) = (width as usize, height as usize);
    let mut cos = [[0f64; 8]; 8];
    for (x, row) in cos.iter_mut().enumerate() {
        for (u, c) in row.iter_mut().enumerate() {
            *c = (((2 * x + 1) * u) as f64 * std::f64::consts::PI / 16.0).cos();
        }
    }
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            let mut blocks = [[0f64; 64]; 3];
            for y in 0..8 {
                for x in 0..8 {
                    let (sx, sy) = ((bx + x).min(w - 1), (by + y).min(h - 1));
                    let p = &rgba[(sy * w + sx) * 4..];
                    let (r, g, b) = (f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
                    blocks[0][y * 8 + x] = 0.299 * r + 0.587 * g + 0.114 * b - 128.0;
                    blocks[1][y * 8 + x] = -0.168_736 * r - 0.331_264 * g + 0.5 * b;
                    blocks[2][y * 8 + x] = 0.5 * r - 0.418_688 * g - 0.081_312 * b;
                }
            }
            for (c, block) in blocks.iter().enumerate() {
                let q = if c == 0 { &qy } else { &qc };
                let mut coef = [0i32; 64];
                for v in 0..8 {
                    for u in 0..8 {
                        let mut sum = 0.0;
                        for y in 0..8 {
                            for x in 0..8 {
                                sum += block[y * 8 + x] * cos[x][u] * cos[y][v];
                            }
                        }
                        let cu = if u == 0 {
                            std::f64::consts::FRAC_1_SQRT_2
                        } else {
                            1.0
                        };
                        let cv = if v == 0 {
                            std::f64::consts::FRAC_1_SQRT_2
                        } else {
                            1.0
                        };
                        let value = 0.25 * cu * cv * sum;
                        coef[v * 8 + u] = (value / f64::from(q[v * 8 + u])).round() as i32;
                    }
                }
                // DC différentiel.
                let diff = coef[0] - prev_dc[c];
                prev_dc[c] = coef[0];
                let (size, bits) = magnitude(diff);
                writer.put(u32::from(size), 4);
                writer.put(bits, size);
                // AC en zigzag.
                let mut run = 0u8;
                for &n in &NATURAL[1..] {
                    let v = coef[n];
                    if v == 0 {
                        run += 1;
                        continue;
                    }
                    while run >= 16 {
                        writer.put(u32::from(ac_code[0xF0]), 8);
                        run -= 16;
                    }
                    let (size, bits) = magnitude(v);
                    writer.put(u32::from(ac_code[((run << 4) | size) as usize]), 8);
                    writer.put(bits, size);
                    run = 0;
                }
                if run > 0 {
                    writer.put(u32::from(ac_code[0x00]), 8);
                }
            }
        }
    }
    out.extend(writer.finish());
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// Catégorie de magnitude JPEG (`size`) et bits complémentés pour les négatifs.
/// Bornée à ±2047 (catégorie ≤ 11) ; avec la qualité 85, AC ≤ 10 en pratique.
fn magnitude(v: i32) -> (u8, u32) {
    let v = v.clamp(-2047, 2047);
    let size = (32 - v.unsigned_abs().leading_zeros()) as u8;
    let bits = if v >= 0 {
        v as u32
    } else {
        (v - 1) as u32 & ((1 << size) - 1)
    };
    (size, bits)
}

/// Écriture de bits MSB d'abord avec bourrage 0xFF 0x00 (entropy-coded segment).
#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u8,
}

impl BitWriter {
    fn put(&mut self, bits: u32, count: u8) {
        for i in (0..count).rev() {
            self.acc = (self.acc << 1) | ((bits >> i) & 1);
            self.n += 1;
            if self.n == 8 {
                self.push_byte();
            }
        }
    }
    fn push_byte(&mut self) {
        let byte = self.acc as u8;
        self.out.push(byte);
        if byte == 0xFF {
            self.out.push(0);
        }
        self.acc = 0;
        self.n = 0;
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            let pad = 8 - self.n;
            self.acc = (self.acc << pad) | ((1 << pad) - 1);
            self.n = 8;
            self.push_byte();
        }
        self.out
    }
}

/// BTT_TFT (`compress_thumbnail_btt_tft`) : une ligne `;` + RGB565 hexadécimal
/// (4 chiffres minuscules par pixel, alpha prémultiplié) terminée par \r\n ;
/// 0020/0841/0861 → 0000 comme le convertisseur BTT.
pub fn encode_btt_tft(rgba: &[u8], width: u32, height: u32) -> String {
    let mut out = String::with_capacity((width as usize * 4 + 3) * height as usize);
    for row in rgba.chunks(width as usize * 4).take(height as usize) {
        out.push(';');
        for px in row.chunks(4) {
            let a = u32::from(px[3]);
            let (r, g, b) = (
                a * u32::from(px[0]) / 255,
                a * u32::from(px[1]) / 255,
                a * u32::from(px[2]) / 255,
            );
            let mut hex = format!("{:04x}", ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3));
            if matches!(hex.as_str(), "0020" | "0841" | "0861") {
                hex = "0000".into();
            }
            out.push_str(&hex);
        }
        out.push_str("\r\n");
    }
    out
}

/// Entrée de palette de `ColPicEncode` (`U16HEAD`) : couleur RGB565, composantes
/// de distance `A0/A1/A2` calculées comme Orca (promotion en `int` : `A1 = val >> 5`
/// et `A2 = val` tronqués à l'octet), nombre d'occurrences.
#[derive(Clone, Copy)]
struct ColPicEntry {
    colo16: u16,
    a: [u8; 3],
    qty: u32,
}

/// `Byte8bitEncode` (Thumbnails.cpp) : RLE indexé par palette (32 couleurs par
/// page, octet `0xE0 | page` au changement de page ; série ≤ 6 sur un octet
/// `(n << 5) | index`, sinon `index` puis `n` ; séries bornées à 255).
fn colpic_byte8bit(src: &[u16], palette: &[u16], max: usize, out: &mut Vec<u8>) {
    let start = out.len();
    let (mut srcindex, mut dotsqty, mut lastid) = (0usize, src.len(), 0u8);
    while dotsqty > 0 {
        let mut dots = 1usize;
        for i in 0..dotsqty - 1 {
            if src[srcindex + i] != src[srcindex + i + 1] {
                break;
            }
            dots += 1;
            if dots == 255 {
                break;
            }
        }
        let temp = palette
            .iter()
            .position(|&c| c == src[srcindex])
            .unwrap_or(0);
        let (tid, sid) = ((temp % 32) as u8, (temp / 32) as u8);
        let mut push = |b: u8| {
            if out.len() - start >= max {
                return false;
            }
            out.push(b);
            true
        };
        if lastid != sid {
            if !push((7u8 << 5).wrapping_add(sid)) {
                return;
            }
            lastid = sid;
        }
        if dots <= 6 {
            if !push(((dots as u8) << 5).wrapping_add(tid)) {
                return;
            }
        } else if !push(tid) || !push(dots as u8) {
            return;
        }
        srcindex += dots;
        dotsqty -= dots;
    }
}

/// `ColPicEncode` : palette (≤ 1024 couleurs, triée par fréquence décroissante,
/// insertion stable comme Orca), réduction à `colorsmax`, en-tête `ColPicHead3`
/// (32 octets, petit-boutiste), palette RGB565, données RLE.
fn colpic_encode(
    color16: &mut [u16],
    picw: u32,
    pich: u32,
    max: usize,
    colorsmax: usize,
) -> Vec<u8> {
    let colorsmax = colorsmax.min(1024);
    let mut list: Vec<ColPicEntry> = Vec::new();
    for &val in color16.iter() {
        if list.len() >= 1024 {
            continue;
        }
        if let Some(e) = list.iter_mut().find(|e| e.colo16 == val) {
            e.qty += 1;
            continue;
        }
        let v = u32::from(val);
        list.push(ColPicEntry {
            colo16: val,
            a: [(v >> 11) as u8, (v >> 5) as u8, v as u8],
            qty: 1,
        });
    }
    for index in 1..list.len() {
        let l0 = list[index];
        if let Some(i) = (0..index).find(|&i| l0.qty >= list[i].qty) {
            list.copy_within(i..index, i + 1);
            list[i] = l0;
        }
    }
    while list.len() > colorsmax {
        let l0 = *list.last().expect("liste non vide");
        let (mut minval, mut fid) = (255i32, 0usize);
        for (i, e) in list.iter().enumerate().take(colorsmax) {
            let chall: i32 = (0..3)
                .map(|k| (i32::from(e.a[k]) - i32::from(l0.a[k])).abs())
                .sum();
            if chall < minval {
                minval = chall;
                fid = i;
            }
        }
        let target = list[fid].colo16;
        for c in color16.iter_mut().filter(|c| **c == l0.colo16) {
            *c = target;
        }
        list.pop();
    }
    let palette: Vec<u16> = list.iter().map(|e| e.colo16).collect();
    let list_size = palette.len() * 2;
    let mut out = vec![0u8; 32];
    out[0] = 3; // encodever
    out[4..8].copy_from_slice(&picw.to_le_bytes());
    out[8..12].copy_from_slice(&pich.to_le_bytes());
    out[12..16].copy_from_slice(&0x05DD_C33Cu32.to_le_bytes());
    out[16..20].copy_from_slice(&(list_size as u32).to_le_bytes());
    for c in &palette {
        out.extend_from_slice(&c.to_le_bytes());
    }
    colpic_byte8bit(
        color16,
        &palette,
        max.saturating_sub(32 + list_size),
        &mut out,
    );
    let color_size = (out.len() - 32 - list_size) as u32;
    out[20..24].copy_from_slice(&color_size.to_le_bytes());
    out
}

/// `ColPic_EncodeStr` : rembourrage à un multiple de 3 (toujours 1 à 3 octets nuls,
/// comme Orca), puis 6 bits → caractère `'0' + v` (`\\` remplacé par `~`).
/// Chaîne vide si la sortie dépasse `max` (Orca renvoie alors 0).
fn colpic_encode_str(
    color16: &mut [u16],
    picw: u32,
    pich: u32,
    max: usize,
    colorsmax: usize,
) -> String {
    let mut bytes = colpic_encode(color16, picw, pich, max, colorsmax);
    let pad = 3 - bytes.len() % 3;
    bytes.extend(std::iter::repeat_n(0u8, pad));
    if bytes.len() * 4 / 3 >= max {
        return String::new();
    }
    let map = |v: u8| -> char {
        let c = v + 48;
        if c == b'\\' {
            '~'
        } else {
            c as char
        }
    };
    let mut out = String::with_capacity(bytes.len() * 4 / 3);
    for t in bytes.chunks(3) {
        out.push(map(t[0] >> 2));
        out.push(map(((t[0] & 3) << 4) + (t[1] >> 4)));
        out.push(map(((t[1] & 15) << 2) + (t[2] >> 6)));
        out.push(map(t[2] & 63));
    }
    out
}

/// `compress_thumbnail_colpic` (Thumbnails.cpp:124-178) sur des pixels RGBA dans
/// l'ordre du tampon OpenGL d'Orca (`ThumbnailData::pixels`, rangées de bas en
/// haut). Taille plafonnée à 512 (ratio conservé) avec la même lecture linéaire
/// du tampon qu'Orca ; pixels transparents → #2E3348 ; palette ≤ 1024.
pub fn encode_colpic_gl(pixels: &[u8], width: u32, height: u32) -> String {
    const MAX_SIZE: u32 = 512;
    let (mut w, mut h) = (width, height);
    if w > MAX_SIZE || h > MAX_SIZE {
        let ratio = f64::from(w) / f64::from(h);
        if ratio > 1.0 {
            w = MAX_SIZE;
            h = (f64::from(MAX_SIZE) / ratio) as u32;
        } else {
            h = MAX_SIZE;
            w = (f64::from(MAX_SIZE) * ratio) as u32;
        }
    }
    let (wu, hu) = (w as usize, h as usize);
    let n = wu * hu;
    let mut color16 = vec![0u16; n];
    let mut time = n;
    for row in 0..hu {
        for col in 0..wu {
            let idx = 4 * (row * wu + wu - col - 1);
            let px = pixels.get(idx..idx + 4).unwrap_or(&[0, 0, 0, 0]);
            let (r, g, b) = if px[3] == 0 {
                (46u16 >> 3, 51u16 >> 2, 72u16 >> 3)
            } else {
                (
                    u16::from(px[0]) >> 3,
                    u16::from(px[1]) >> 2,
                    u16::from(px[2]) >> 3,
                )
            };
            time -= 1;
            color16[time] = (r << 11) | (g << 5) | b;
        }
    }
    colpic_encode_str(&mut color16, w, h, n * 10, 1024)
}

/// Section de vignettes à insérer (vide si aucune n'est demandée/encodable).
pub fn thumbnails_section(model: &Model, config: &DynamicPrintConfig) -> String {
    let list = thumbnail_list(
        &config_text(config, "thumbnails"),
        &config_text(config, "thumbnails_format"),
    );
    if list.is_empty() {
        return String::new();
    }
    let triangles = world_triangles(model);
    let colour = parse_colour(&config_text(config, "filament_colour"));
    let mut out = String::new();
    let count = list.len();
    let mut first_colpic = true;
    for (i, (format, width, height)) in list.into_iter().enumerate() {
        let rgba = render_rgba(&triangles, colour, width, height);
        let (tag, bytes) = match format {
            ThumbnailFormat::Png => ("thumbnail", encode_png(&rgba, width, height)),
            ThumbnailFormat::Jpg => ("thumbnail_JPG", encode_jpg(&rgba, width, height)),
            ThumbnailFormat::Qoi => ("thumbnail_QOI", encode_qoi(&rgba, width, height)),
            ThumbnailFormat::BttTft => {
                // Thumbnails.hpp:63-68 : en-tête « ;WWWWHHHH » (hex), données brutes,
                // fin « ; bigtree thumbnail end » après la dernière entrée de la liste.
                out.push_str(&format!(";{width:04x}{height:04x}\r\n"));
                out.push_str(&encode_btt_tft(&rgba, width, height));
                if i + 1 == count {
                    out.push_str("; bigtree thumbnail end\r\n\r\n");
                }
                continue;
            }
            ThumbnailFormat::ColPic => {
                // Thumbnails.hpp:70-77 : « ;gimage: » pour le premier, « ;simage: » ensuite.
                let mut gl = Vec::with_capacity(rgba.len());
                for row in rgba.chunks(width as usize * 4).rev() {
                    gl.extend_from_slice(row);
                }
                let tag = if first_colpic { "gimage" } else { "simage" };
                first_colpic = false;
                out.push_str(&format!(
                    "\n\n;{tag}:{}\n\n",
                    encode_colpic_gl(&gl, width, height)
                ));
                continue;
            }
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        out.push_str("; THUMBNAIL_BLOCK_START\n");
        out.push_str(&format!(
            "\n;\n; {tag} begin {width}x{height} {}\n",
            encoded.len()
        ));
        for row in encoded.as_bytes().chunks(78) {
            out.push_str("; ");
            out.push_str(std::str::from_utf8(row).unwrap_or_default());
            out.push('\n');
        }
        out.push_str(&format!("; {tag} end\n"));
        out.push_str("; THUMBNAIL_BLOCK_END\n\n");
    }
    out
}

/// Insère la section après le bloc d'en-tête (`; HEADER_BLOCK_END` + ligne vide),
/// comme `GCode::_do_export` ; G-code inchangé sans section ou sans en-tête.
pub fn insert_section(gcode: &str, section: &str) -> String {
    if section.is_empty() {
        return gcode.to_string();
    }
    let Some(pos) = gcode.find("; HEADER_BLOCK_END\n") else {
        return gcode.to_string();
    };
    let mut at = pos + "; HEADER_BLOCK_END\n".len();
    if gcode[at..].starts_with('\n') {
        at += 1;
    }
    let mut out = String::with_capacity(gcode.len() + section.len());
    out.push_str(&gcode[..at]);
    out.push_str(section);
    out.push_str(&gcode[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thumbnail_list_with_default_extension() {
        assert_eq!(
            thumbnail_list("48x48/PNG, 96x96, 300x300/qoi", "QOI"),
            vec![
                (ThumbnailFormat::Png, 48, 48),
                (ThumbnailFormat::Qoi, 96, 96),
                (ThumbnailFormat::Qoi, 300, 300)
            ]
        );
        assert!(thumbnail_list("", "PNG").is_empty());
        assert!(thumbnail_list("0x10/PNG,abc", "PNG").is_empty());
    }

    #[test]
    fn png_and_qoi_headers_are_valid() {
        let rgba = vec![255u8; 4 * 4 * 3];
        let png = encode_png(&rgba, 4, 3);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 4);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3);
        let qoi = encode_qoi(&rgba, 4, 3);
        assert_eq!(&qoi[..4], b"qoif");
        assert!(qoi.ends_with(&[0, 0, 0, 0, 0, 0, 0, 1]));
    }

    #[test]
    fn jpg_is_a_decodable_baseline_jfif() {
        let mut rgba = vec![0u8; 20 * 13 * 4];
        for (i, px) in rgba.chunks_mut(4).enumerate() {
            px.copy_from_slice(&[(i * 7) as u8, 200, (i * 3) as u8, 255]);
        }
        let jpg = encode_jpg(&rgba, 20, 13);
        assert_eq!(&jpg[..4], &[0xFF, 0xD8, 0xFF, 0xE0]);
        assert!(jpg.ends_with(&[0xFF, 0xD9]));
        let sof = jpg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        assert_eq!(u16::from_be_bytes([jpg[sof + 5], jpg[sof + 6]]), 13);
        assert_eq!(u16::from_be_bytes([jpg[sof + 7], jpg[sof + 8]]), 20);
        if let Ok(dir) = std::env::var("THUMBNAIL_JPG_DUMP") {
            std::fs::write(std::path::Path::new(&dir).join("t.jpg"), &jpg).unwrap();
        }
    }

    #[test]
    fn btt_tft_rows_are_rgb565_hex() {
        let rgba = [
            255u8, 255, 255, 255, 0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 255,
        ];
        assert_eq!(encode_btt_tft(&rgba, 2, 2), ";ffff0000\r\n;f8000000\r\n");
    }

    /// Image de test identique au harnais C++ (staging/params-metadata-2/colpic,
    /// Thumbnails.cpp:262-529 compilé tel quel) : couleur `(i/3)%40`, un pixel sur
    /// 17 transparent ; variante « flat » : 290 pixels d'une couleur puis 10 d'une autre.
    fn colpic_pixels(width: u32, height: u32, flat: bool) -> Vec<u8> {
        let mut px = Vec::new();
        for i in 0..(width * height) as usize {
            let c = if flat {
                if i < 290 {
                    7
                } else {
                    9
                }
            } else {
                (i / 3) % 40
            };
            px.extend_from_slice(&[
                (c * 6) as u8,
                (255 - c * 5) as u8,
                ((c * 37) % 256) as u8,
                if !flat && i % 17 == 0 { 0 } else { 255 },
            ]);
        }
        px
    }

    #[test]
    fn colpic_matches_orca_cpp_reference() {
        // Sorties de `./ref W H [flat]` (ColPic_EncodeStr d'Orca 2.4.1).
        let cases = [
            (5, 3, false, "0`0000D000030000?<?M1@`00008000000000000002Y3l@7DQn=5n07RBU3HR5P8bE4@@00"),
            (12, 10, false, "0`0000`0000:0000?<?M1E8000180000000000000029:HdGZ@o41n0^6bMB7fhnRCJU=ZEM`5GlCAM>5gDcKFYUJXbQO=alg:?hVa>D;i@_^dZcIZ^1d]S2m<8?hR_JA][P1cLODTI>IHF4PJ^mb_CYhF3POgkQB>0POG`PhDOPNn4Vh23Q9^1jNGQgMWEdLg;QAN0PLG0PhDCPKfi]K6]ZJFQWhB?P8>4Sh6KQ@^0PIF@PhD7PHf9Q0000"),
            (20, 15, true, "0`0001@0000?0000?<?M1@@00008000000000000003P;XTf00X12P3o01T0"),
        ];
        for (w, h, flat, expected) in cases {
            assert_eq!(
                encode_colpic_gl(&colpic_pixels(w, h, flat), w, h),
                expected,
                "{w}x{h} flat={flat}"
            );
        }
    }

    #[test]
    fn inserts_after_the_header_block() {
        let g = "; HEADER_BLOCK_START\n; HEADER_BLOCK_END\n\n; external perimeters\nG1\n";
        let out = insert_section(g, "; THUMBNAIL_BLOCK_START\nX\n; THUMBNAIL_BLOCK_END\n\n");
        assert_eq!(
            out,
            "; HEADER_BLOCK_START\n; HEADER_BLOCK_END\n\n; THUMBNAIL_BLOCK_START\nX\n; THUMBNAIL_BLOCK_END\n\n; external perimeters\nG1\n"
        );
    }
}
