//! Сплошные обои в стиле приложения: цвет и едва заметный узор, рисуются в родном разрешении
//! экрана и кодируются в PNG. Таблица цветов лежит в JSON: тот же файл читает интерфейс.

use crate::appearance::{parse_hex, AppearanceError};

const SOLID_JSON: &str = include_str!("../assets/solid-wallpapers.json");
/// Глубина узора: чёрный слой с прозрачностью 5,5 % поверх цвета.
const PATTERN_DARKEN: f64 = 0.055;
const MIN_SIDE: u32 = 640;
const MAX_SIDE: u32 = 8192;

/// Запись таблицы: `pattern` — `none`, `dots`, `lines` или `grid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidWallpaper {
    pub id: String,
    pub color: String,
    pub pattern: String,
}

pub fn solid_wallpapers() -> Vec<SolidWallpaper> {
    let v: serde_json::Value =
        serde_json::from_str(SOLID_JSON).expect("solid-wallpapers.json проверен тестами");
    v.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    Some(SolidWallpaper {
                        id: i["id"].as_str()?.to_string(),
                        color: i["color"].as_str()?.to_string(),
                        pattern: i["pattern"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Закрашивает ли узор `pattern` точку (`x`, `y`) в сетке 3840×2160.
fn darkened(pattern: &str, x: f64, y: f64) -> bool {
    match pattern {
        "dots" => {
            let (dx, dy) = ((x - 24.0).rem_euclid(100.0), (y - 24.0).rem_euclid(100.0));
            let (dx, dy) = (dx.min(100.0 - dx), dy.min(100.0 - dy));
            dx.hypot(dy) <= 9.0
        }
        "lines" => ((x + y - 160.0 + 100.0).rem_euclid(200.0) - 100.0).abs() <= 11.3,
        "grid" => x.rem_euclid(180.0) < 4.0 || y.rem_euclid(180.0) < 4.0,
        _ => false,
    }
}

/// PNG сплошных обоев `id` размером `width`×`height`.
pub fn render_solid(id: &str, width: u32, height: u32) -> Result<Vec<u8>, AppearanceError> {
    let item = solid_wallpapers()
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| AppearanceError::Invalid("неизвестные сплошные обои".into()))?;
    if !(MIN_SIDE..=MAX_SIDE).contains(&width) || !(MIN_SIDE..=MAX_SIDE).contains(&height) {
        return Err(AppearanceError::Invalid("недопустимый размер обоев".into()));
    }
    let (r, g, b) = parse_hex(&item.color)?;
    let base = [r, g, b];
    let dark = base.map(|c| (f64::from(c) * (1.0 - PATTERN_DARKEN)).round() as u8);
    // Узор задан для 3840×2160: масштабируем координаты к этой сетке.
    let scale = 3840.0 / f64::from(width);
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 3);
    for y in 0..height {
        let gy = (f64::from(y) + 0.5) * scale;
        for x in 0..width {
            let on =
                item.pattern != "none" && darkened(&item.pattern, (f64::from(x) + 0.5) * scale, gy);
            pixels.extend_from_slice(if on { &dark } else { &base });
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| AppearanceError::Wallpaper(e.to_string()))?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| AppearanceError::Wallpaper(e.to_string()))?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        (info.width, info.height, buf[..info.buffer_size()].to_vec())
    }

    #[test]
    fn table_has_unique_ids_and_valid_colours_acid_first() {
        let all = solid_wallpapers();
        assert!(all.len() >= 14);
        let mut ids: Vec<_> = all.iter().map(|w| w.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        for w in &all {
            assert!(parse_hex(&w.color).is_ok(), "{}", w.id);
            assert!(w.id.starts_with("solid-"));
        }
        assert!(all[0].id.starts_with("solid-acid-"));
    }

    #[test]
    fn png_has_the_requested_native_size_and_the_colour() {
        let (w, h) = (1280, 720);
        let (dw, dh, px) = decode(&render_solid("solid-acid-01", w, h).unwrap());
        assert_eq!((dw, dh), (w, h));
        assert_eq!(&px[..3], &[0xB4, 0xFF, 0x00]);
        let white = decode(&render_solid("solid-white", w, h).unwrap()).2;
        assert!(white.iter().all(|&c| c == 255));
    }

    #[test]
    fn patterns_darken_some_pixels_slightly() {
        let px = decode(&render_solid("solid-acid-13", 1920, 1080).unwrap()).2;
        let base = [0xF5u8, 0xFF, 0x00];
        let darker = px.chunks(3).filter(|p| p != &base).count();
        assert!(darker > 1000, "узор «сетка» виден");
        assert!(px.chunks(3).all(|p| p[0] >= 230), "узор едва заметен");
    }

    #[test]
    fn bad_id_or_size_is_refused() {
        assert!(render_solid("solid-nope", 1920, 1080).is_err());
        assert!(render_solid("photo-1", 1920, 1080).is_err());
        assert!(render_solid("solid-black", 100, 100).is_err());
        assert!(render_solid("solid-black", 20000, 1080).is_err());
    }
}
