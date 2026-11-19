//! Сплошные обои в стиле приложения: цвет страницы и тот же узор pop-pattern, что рисует фон
//! приложения (плитка 320 px, непрозрачность 0,07). Рисуются в родном разрешении экрана и
//! кодируются в PNG. Таблица цветов лежит в JSON: тот же файл читает интерфейс. Узор вшит в
//! приложение (`include_bytes!`), отдельных файлов рядом не нужно.

use std::sync::OnceLock;

use crate::appearance::{parse_hex, AppearanceError};

const SOLID_JSON: &str = include_str!("../assets/solid-wallpapers.json");
/// Узор фона приложения: чёрный рисунок на прозрачном.
const PATTERN_PNG: &[u8] = include_bytes!("../../../apps/desktop/src/assets/pop-pattern.png");
/// Как в приложении (`PATTERN` в `tokens.ts`): плитка узора на экране 1920 px и прозрачность слоя.
const PATTERN_TILE: f64 = 320.0;
const PATTERN_OPACITY: f64 = 0.07;
const MIN_SIDE: u32 = 640;
const MAX_SIDE: u32 = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolidWallpaper {
    pub id: String,
    pub color: String,
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
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Узор в исходном размере: RGBA, 8 бит.
struct Pattern {
    size: usize,
    rgba: Vec<u8>,
}

fn pattern() -> Result<&'static Pattern, AppearanceError> {
    static CELL: OnceLock<Result<Pattern, String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(PATTERN_PNG));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf =
            vec![0; reader.output_buffer_size().ok_or("узор слишком велик")?];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        if info.width != info.height {
            return Err("узор должен быть квадратным".into());
        }
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
            png::ColorType::Rgb => buf[..info.buffer_size()]
                .chunks(3)
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buf[..info.buffer_size()]
                .chunks(2)
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            png::ColorType::Grayscale => buf[..info.buffer_size()]
                .iter()
                .flat_map(|&g| [g, g, g, 255])
                .collect(),
            png::ColorType::Indexed => return Err("неожиданный формат узора".into()),
        };
        Ok(Pattern {
            size: info.width as usize,
            rgba,
        })
    })
    .as_ref()
    .map_err(|e| AppearanceError::Wallpaper(e.clone()))
}

/// Плитка узора `tile`×`tile`, смешанная с цветом `base`: усреднение по площади (без ряби при
/// уменьшении), цвет слоя берётся с учётом прозрачности и непрозрачности слоя, как в CSS.
fn blended_tile(base: [u8; 3], tile: usize) -> Result<Vec<u8>, AppearanceError> {
    let pat = pattern()?;
    let ratio = pat.size as f64 / tile as f64;
    let mut out = vec![0u8; tile * tile * 3];
    for ty in 0..tile {
        let (y0, y1) = span(ty, ratio, pat.size);
        for tx in 0..tile {
            let (x0, x1) = span(tx, ratio, pat.size);
            let (mut a_sum, mut c_sum, mut n) = ([0.0f64; 3], 0.0f64, 0.0f64);
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let i = (sy * pat.size + sx) * 4;
                    let a = f64::from(pat.rgba[i + 3]) / 255.0;
                    for (k, sum) in a_sum.iter_mut().enumerate() {
                        *sum += f64::from(pat.rgba[i + k]) * a;
                    }
                    c_sum += a;
                    n += 1.0;
                }
            }
            // Средний цвет узора (с весом прозрачности) и средняя прозрачность клетки.
            let cover = c_sum / n;
            let k = cover * PATTERN_OPACITY;
            let o = (ty * tile + tx) * 3;
            for c in 0..3 {
                let layer = if c_sum > 0.0 { a_sum[c] / c_sum } else { 0.0 };
                let v = f64::from(base[c]) * (1.0 - k) + layer * k;
                out[o + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    Ok(out)
}

/// Диапазон исходных пикселей `[from, to)` для пикселя `i` плитки (минимум один).
fn span(i: usize, ratio: f64, max: usize) -> (usize, usize) {
    let from = ((i as f64 * ratio).floor() as usize).min(max - 1);
    let to = (((i + 1) as f64 * ratio).ceil() as usize).clamp(from + 1, max);
    (from, to)
}

/// PNG сплошных обоев `id` размером `width`×`height`: цвет и узор фона приложения.
pub fn render_solid(id: &str, width: u32, height: u32) -> Result<Vec<u8>, AppearanceError> {
    let item = solid_wallpapers()
        .into_iter()
        .find(|w| w.id == id)
        .ok_or_else(|| AppearanceError::Invalid("неизвестные сплошные обои".into()))?;
    if !(MIN_SIDE..=MAX_SIDE).contains(&width) || !(MIN_SIDE..=MAX_SIDE).contains(&height) {
        return Err(AppearanceError::Invalid("недопустимый размер обоев".into()));
    }
    let (r, g, b) = parse_hex(&item.color)?;
    // Плитка растёт с экраном: на 4K узор выглядит так же, как на 1920 px.
    let tile = (PATTERN_TILE * (f64::from(width) / 1920.0).max(0.75)).round() as usize;
    let cell = blended_tile([r, g, b], tile)?;
    let (w, h) = (width as usize, height as usize);
    let mut pixels = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let row = &cell[(y % tile) * tile * 3..][..tile * 3];
        let mut x = 0;
        while x < w {
            let n = tile.min(w - x);
            pixels.extend_from_slice(&row[..n * 3]);
            x += n;
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
    fn table_has_exactly_seven_tiles_four_acid_black_white_red() {
        let all = solid_wallpapers();
        let ids: Vec<_> = all.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(all.len(), 7, "{ids:?}");
        assert_eq!(
            ids.iter().filter(|i| i.starts_with("solid-acid-")).count(),
            4
        );
        for fixed in ["solid-black", "solid-white", "solid-red"] {
            assert!(ids.contains(&fixed), "{fixed}");
        }
        for w in &all {
            assert!(parse_hex(&w.color).is_ok(), "{}", w.id);
        }
        assert!(all[0].id.starts_with("solid-acid-"));
    }

    #[test]
    fn png_has_the_requested_native_size() {
        let (dw, dh, px) = decode(&render_solid("solid-acid-lime", 1280, 720).unwrap());
        assert_eq!((dw, dh), (1280, 720));
        assert_eq!(px.len(), 1280 * 720 * 3);
    }

    #[test]
    fn the_app_pattern_darkens_the_colour_slightly_and_repeats() {
        let (w, h) = (1920usize, 1080usize);
        let px = decode(&render_solid("solid-acid-lime", w as u32, h as u32).unwrap()).2;
        let base = [0xB4u8, 0xFF, 0x00];
        let darker = px.chunks(3).filter(|p| p[0] < base[0]).count();
        assert!(darker > w * h / 40, "узор виден: {darker}");
        // Узор едва заметен: не темнее чем на ~8 %.
        assert!(px.chunks(3).all(|p| p[0] >= 0xB4 - 15 && p[1] >= 235));
        // Плитка 320 px повторяется без швов.
        let at = |x: usize, y: usize| &px[(y * w + x) * 3..][..3];
        assert_eq!(at(10, 20), at(330, 340));
        assert_eq!(at(77, 5), at(77 + 640, 5 + 320));
    }

    #[test]
    fn white_and_black_get_the_pattern_too() {
        let white = decode(&render_solid("solid-white", 1280, 720).unwrap()).2;
        assert!(white.chunks(3).any(|p| p[0] < 255), "узор на белом");
        let black = decode(&render_solid("solid-black", 1280, 720).unwrap()).2;
        assert!(
            black.chunks(3).all(|p| p[0] <= 8),
            "чёрный узор не светлеет"
        );
    }

    #[test]
    fn bad_id_or_size_is_refused() {
        assert!(render_solid("solid-nope", 1920, 1080).is_err());
        assert!(render_solid("photo-1", 1920, 1080).is_err());
        assert!(render_solid("solid-black", 100, 100).is_err());
        assert!(render_solid("solid-black", 20000, 1080).is_err());
    }
}
