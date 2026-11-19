//! Сплошные обои в стиле приложения: цвет и едва заметный узор, рисуются в родном разрешении
//! экрана и кодируются в PNG. Таблица цветов лежит в JSON: тот же файл читает интерфейс.

use crate::appearance::{parse_hex, AppearanceError};

const SOLID_JSON: &str = include_str!("../assets/solid-wallpapers.json");
/// Глубина узора: чёрный слой с прозрачностью 5,5 % поверх цвета.
const PATTERN_DARKEN: f64 = 0.055;
/// Узор приложения (тот же, что фон окна) и его прозрачность поверх цвета.
const POP_PNG: &[u8] = include_bytes!("../assets/pop-pattern.png");
const POP_OPACITY: f64 = 0.07;
/// Плитка узора при высоте экрана 1080 px (как в окне: 320 px).
const POP_TILE_1080: f64 = 320.0;

/// RGBA-пиксели узора: (ширина, высота, данные).
fn pop_pattern() -> Result<(u32, u32, Vec<u8>), AppearanceError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(POP_PNG));
    let mut reader = decoder
        .read_info()
        .map_err(|e| AppearanceError::Wallpaper(e.to_string()))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| AppearanceError::Wallpaper("узор".into()))?;
    let mut buf = vec![0; size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| AppearanceError::Wallpaper(e.to_string()))?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(AppearanceError::Wallpaper("формат узора".into()));
    }
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, buf))
}

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
    if item.pattern == "pop" {
        let (pw, ph, pat) = pop_pattern()?;
        let tile = POP_TILE_1080 * f64::from(height) / 1080.0;
        let (sx, sy) = (f64::from(pw) / tile, f64::from(ph) / tile);
        for y in 0..height {
            let py = ((f64::from(y) * sy) as u32 % ph) as usize;
            for x in 0..width {
                let px = ((f64::from(x) * sx) as u32 % pw) as usize;
                let i = (py * pw as usize + px) * 4;
                let a = f64::from(pat[i + 3]) / 255.0 * POP_OPACITY;
                for c in 0..3 {
                    let v = f64::from(base[c]) * (1.0 - a) + f64::from(pat[i + c]) * a;
                    pixels.push(v.round() as u8);
                }
            }
        }
    }
    for y in (0..height).filter(|_| item.pattern != "pop") {
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
        assert_eq!(all.len(), 7);
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
        assert!(px.chunks(3).any(|p| p == [0xB4, 0xFF, 0x00]));
    }

    #[test]
    fn patterns_darken_some_pixels_slightly() {
        let px = decode(&render_solid("solid-red", 1920, 1080).unwrap()).2;
        let base = [0xFFu8, 0x1E, 0x1E];
        let changed = px.chunks(3).filter(|p| p != &base).count();
        assert!(changed > 1000, "узор приложения виден");
        assert!(px.chunks(3).all(|p| p[0] >= 220), "узор едва заметен");
    }

    #[test]
    fn bad_id_or_size_is_refused() {
        assert!(render_solid("solid-nope", 1920, 1080).is_err());
        assert!(render_solid("photo-1", 1920, 1080).is_err());
        assert!(render_solid("solid-black", 100, 100).is_err());
        assert!(render_solid("solid-black", 20000, 1080).is_err());
    }
}
