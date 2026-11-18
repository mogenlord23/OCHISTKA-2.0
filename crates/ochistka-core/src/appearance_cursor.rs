//! Рисунок курсора Windows: 32×32, PNG внутри `.cur`, остриё стрелки в «горячей точке».
//! Растеризация и кодирование PNG написаны вручную (без зависимостей и без unsafe).
//! Данные (цвет, второй цвет, узор) приходят из таблицы курсоров интерфейса;
//! здесь они проверяются. Файлы пишет порт, в тестах на диск ничего не попадает.

use crate::appearance::{parse_hex, AppearanceError};

pub const SIZE: usize = 32;
const SAMPLES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Plain,
    Stripes,
    Dots,
    Checker,
    Cross,
    /// Анимированная схема: стрелка плавно меняет цвет (`.ani`), остальные курсоры однотонные.
    Pulse,
}

impl Pattern {
    pub fn parse(name: &str) -> Result<Self, AppearanceError> {
        Ok(match name {
            "plain" => Self::Plain,
            "stripes" => Self::Stripes,
            "dots" => Self::Dots,
            "checker" => Self::Checker,
            "cross" => Self::Cross,
            "pulse" => Self::Pulse,
            _ => return Err(AppearanceError::Invalid("неизвестный узор курсора".into())),
        })
    }
}

/// Роль курсора в схеме Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Arrow,
    IBeam,
}

impl Role {
    pub const ALL: [Role; 2] = [Role::Arrow, Role::IBeam];

    /// Имя значения в `HKCU\Control Panel\Cursors`.
    pub fn registry_name(self) -> &'static str {
        match self {
            Role::Arrow => "Arrow",
            Role::IBeam => "IBeam",
        }
    }

    /// Контур в сетке 32×32 (до центровки).
    fn outline(self) -> Vec<(f64, f64)> {
        match self {
            // Классический указатель: остриё в первой точке.
            Role::Arrow => [
                (0.0, 0.0),
                (0.0, 16.5),
                (3.8, 12.8),
                (6.6, 19.4),
                (9.2, 18.3),
                (6.4, 11.8),
                (11.6, 11.8),
            ]
            .iter()
            .map(|&(x, y)| (x * 1.15, y * 1.15))
            .collect(),
            Role::IBeam => vec![
                (10.0, 4.0),
                (22.0, 4.0),
                (22.0, 7.0),
                (17.5, 7.0),
                (17.5, 25.0),
                (22.0, 25.0),
                (22.0, 28.0),
                (10.0, 28.0),
                (10.0, 25.0),
                (14.5, 25.0),
                (14.5, 7.0),
                (10.0, 7.0),
            ],
        }
    }
}

/// Контур, сдвинутый так, чтобы рисунок вместе с обводкой стоял по центру 32×32.
/// Возвращает точки и горячую точку.
pub fn placed_outline(role: Role) -> (Vec<(f64, f64)>, (u16, u16)) {
    let pts = role.outline();
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in &pts {
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    let dx = (SIZE as f64 - (maxx - minx)) / 2.0 - minx;
    let dy = (SIZE as f64 - (maxy - miny)) / 2.0 - miny;
    let placed: Vec<(f64, f64)> = pts.iter().map(|&(x, y)| (x + dx, y + dy)).collect();
    let hot = match role {
        Role::Arrow => (placed[0].0.round() as u16, placed[0].1.round() as u16),
        Role::IBeam => ((SIZE / 2) as u16, (SIZE / 2) as u16),
    };
    (placed, hot)
}

/// Знаковое расстояние до контура: меньше нуля внутри.
fn signed_distance(poly: &[(f64, f64)], px: f64, py: f64) -> f64 {
    let mut inside = false;
    let mut best = f64::MAX;
    for i in 0..poly.len() {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % poly.len()];
        if (y1 > py) != (y2 > py) && px < (x2 - x1) * (py - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
        let (ex, ey) = (x2 - x1, y2 - y1);
        let len2 = ex * ex + ey * ey;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((px - x1) * ex + (py - y1) * ey) / len2).clamp(0.0, 1.0)
        };
        let d = ((px - x1 - t * ex).powi(2) + (py - y1 - t * ey).powi(2)).sqrt();
        best = best.min(d);
    }
    if inside {
        -best
    } else {
        best
    }
}

/// Цвет заливки в точке: узор с ячейкой 8 пикселей.
fn fill_color(pattern: Pattern, main: [f64; 3], second: [f64; 3], x: f64, y: f64) -> [f64; 3] {
    const WHITE: [f64; 3] = [255.0, 255.0, 255.0];
    let cx = x.rem_euclid(8.0);
    let cy = y.rem_euclid(8.0);
    match pattern {
        Pattern::Plain | Pattern::Pulse => main,
        Pattern::Stripes => {
            if ((x + y) / std::f64::consts::SQRT_2).rem_euclid(8.0) < 3.0 {
                second
            } else {
                main
            }
        }
        Pattern::Dots => {
            if (cx - 2.0).hypot(cy - 2.0) <= 1.5 {
                second
            } else if (cx - 6.0).hypot(cy - 6.0) <= 1.0 {
                WHITE
            } else {
                main
            }
        }
        Pattern::Checker => {
            if (cx < 4.0) == (cy < 4.0) {
                second
            } else {
                main
            }
        }
        Pattern::Cross => {
            if (cx - 4.0).abs() <= 0.5 {
                WHITE
            } else if (cy - 4.0).abs() <= 1.25 {
                second
            } else {
                main
            }
        }
    }
}

/// Растр RGBA 32×32 (не премультиплицированный).
pub fn render(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
) -> Result<Vec<u8>, AppearanceError> {
    let (r, g, b) = parse_hex(color)?;
    let (r2, g2, b2) = parse_hex(secondary)?;
    let main = [f64::from(r), f64::from(g), f64::from(b)];
    let second = [f64::from(r2), f64::from(g2), f64::from(b2)];
    let (poly, _) = placed_outline(role);
    let mut out = vec![0u8; SIZE * SIZE * 4];
    let total = (SAMPLES * SAMPLES) as f64;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let (mut acc, mut alpha) = ([0.0f64; 3], 0.0f64);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = px as f64 + (sx as f64 + 0.5) / SAMPLES as f64;
                    let y = py as f64 + (sy as f64 + 0.5) / SAMPLES as f64;
                    let s = signed_distance(&poly, x, y);
                    let c = if s <= -0.7 {
                        fill_color(pattern, main, second, x, y)
                    } else if s <= 0.7 {
                        [0.0; 3]
                    } else if s <= 2.0 {
                        [255.0; 3]
                    } else {
                        continue;
                    };
                    for k in 0..3 {
                        acc[k] += c[k];
                    }
                    alpha += 1.0;
                }
            }
            let i = (py * SIZE + px) * 4;
            if alpha > 0.0 {
                for k in 0..3 {
                    out[i + k] = (acc[k] / alpha).round() as u8;
                }
                out[i + 3] = (alpha / total * 255.0).round() as u8;
            }
        }
    }
    Ok(out)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
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

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let mut crc_input = kind.to_vec();
    crc_input.extend_from_slice(body);
    out.extend_from_slice(&crc_input);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// PNG RGBA 8 бит; сжатие zlib «stored» (рисунок крошечный).
pub fn encode_png(rgba: &[u8], size: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity((size * 4 + 1) * size);
    for row in rgba.chunks(size * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&(size as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// Файл `.cur` с одним изображением 32×32 (PNG внутри) и горячей точкой.
pub fn build_cur(
    role: Role,
    pattern: Pattern,
    color: &str,
    secondary: &str,
) -> Result<Vec<u8>, AppearanceError> {
    let png = encode_png(&render(role, pattern, color, secondary)?, SIZE);
    let (_, (hx, hy)) = placed_outline(role);
    let mut cur = vec![0, 0, 2, 0, 1, 0, SIZE as u8, SIZE as u8, 0, 0];
    cur.extend_from_slice(&hx.to_le_bytes());
    cur.extend_from_slice(&hy.to_le_bytes());
    cur.extend_from_slice(&(png.len() as u32).to_le_bytes());
    cur.extend_from_slice(&22u32.to_le_bytes());
    cur.extend_from_slice(&png);
    Ok(cur)
}

/// Кадров в анимации стрелки: цвет, середина, второй цвет, середина.
pub const PULSE_FRAMES: usize = 4;
/// Длительность кадра в «джиффи» (1/60 с): весь цикл около 1,3 секунды.
const PULSE_JIFFIES: u32 = 20;

/// Смесь двух цветов `#RRGGBB`; `t` от 0 (первый) до 1 (второй).
pub fn mix_hex(a: &str, b: &str, t: f64) -> Result<String, AppearanceError> {
    let (r1, g1, b1) = parse_hex(a)?;
    let (r2, g2, b2) = parse_hex(b)?;
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    Ok(format!(
        "#{:02X}{:02X}{:02X}",
        m(r1, r2),
        m(g1, g2),
        m(b1, b2)
    ))
}

/// Расширение файла роли: анимированная стрелка пишется как `.ani`, остальное как `.cur`.
pub fn file_extension(role: Role, pattern: Pattern) -> &'static str {
    if role == Role::Arrow && pattern == Pattern::Pulse {
        "ani"
    } else {
        "cur"
    }
}

fn riff_chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(kind);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

/// Файл `.ani` (RIFF ACON): кадры `.cur` с плавной сменой цвета между `color` и `secondary`.
pub fn build_ani(role: Role, color: &str, secondary: &str) -> Result<Vec<u8>, AppearanceError> {
    let steps = [0.0, 0.5, 1.0, 0.5];
    let mut frames = Vec::new();
    for t in steps {
        let c = mix_hex(color, secondary, t)?;
        let cur = build_cur(role, Pattern::Plain, &c, &c)?;
        riff_chunk(&mut frames, b"icon", &cur);
    }
    let mut list = b"fram".to_vec();
    list.extend_from_slice(&frames);
    let mut anih = Vec::new();
    for v in [
        36u32,
        PULSE_FRAMES as u32,
        PULSE_FRAMES as u32,
        0,
        0,
        0,
        0,
        PULSE_JIFFIES,
        1,
    ] {
        anih.extend_from_slice(&v.to_le_bytes());
    }
    let mut body = b"ACON".to_vec();
    riff_chunk(&mut body, b"anih", &anih);
    riff_chunk(&mut body, b"LIST", &list);
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_is_centred_and_hotspot_is_the_tip() {
        let (poly, hot) = placed_outline(Role::Arrow);
        let minx = poly.iter().map(|p| p.0).fold(f64::MAX, f64::min);
        let maxx = poly.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        let miny = poly.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let maxy = poly.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        assert!((minx + maxx - 32.0).abs() < 0.01 && (miny + maxy - 32.0).abs() < 0.01);
        assert_eq!(
            (f64::from(hot.0), f64::from(hot.1)),
            (minx.round(), miny.round())
        );
    }

    #[test]
    fn render_has_opaque_fill_and_transparent_corner() {
        let rgba = render(Role::Arrow, Pattern::Plain, "#FF0000", "#FF0000").unwrap();
        assert_eq!(rgba.len(), 32 * 32 * 4);
        assert_eq!(rgba[3], 0, "угол прозрачный");
        assert!(
            rgba.chunks(4).any(|p| p == [255, 0, 0, 255]),
            "есть красная заливка"
        );
    }

    #[test]
    fn cur_header_and_png_are_valid() {
        let cur = build_cur(Role::IBeam, Pattern::Stripes, "#B4FF00", "#FF009D").unwrap();
        assert_eq!(&cur[..6], &[0, 0, 2, 0, 1, 0]);
        assert_eq!((cur[6], cur[7]), (32, 32));
        let size = u32::from_le_bytes(cur[14..18].try_into().unwrap()) as usize;
        assert_eq!(cur.len(), 22 + size);
        assert_eq!(
            &cur[22..30],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        assert_eq!(&cur[cur.len() - 8..cur.len() - 4], b"IEND");
    }

    #[test]
    fn pulse_scheme_makes_a_valid_ani_for_the_arrow() {
        let ani = build_ani(Role::Arrow, "#FF009D", "#00FFFF").unwrap();
        assert_eq!(&ani[..4], b"RIFF");
        assert_eq!(&ani[8..12], b"ACON");
        let riff = u32::from_le_bytes(ani[4..8].try_into().unwrap()) as usize;
        assert_eq!(ani.len(), 8 + riff);
        assert_eq!(&ani[12..16], b"anih");
        let frames = u32::from_le_bytes(ani[12 + 8 + 4..12 + 8 + 8].try_into().unwrap());
        assert_eq!(frames as usize, PULSE_FRAMES);
        assert!(ani.windows(4).any(|w| w == b"fram"));
        assert_eq!(
            ani.windows(4).filter(|w| *w == b"icon").count(),
            PULSE_FRAMES
        );
        assert_eq!(Pattern::parse("pulse").unwrap(), Pattern::Pulse);
        assert_eq!(file_extension(Role::Arrow, Pattern::Pulse), "ani");
        assert_eq!(file_extension(Role::IBeam, Pattern::Pulse), "cur");
        assert_eq!(file_extension(Role::Arrow, Pattern::Plain), "cur");
    }

    #[test]
    fn pulse_frames_differ_and_mix_is_exact() {
        assert_eq!(mix_hex("#000000", "#FFFFFF", 0.0).unwrap(), "#000000");
        assert_eq!(mix_hex("#000000", "#FFFFFF", 1.0).unwrap(), "#FFFFFF");
        let a = build_ani(Role::Arrow, "#FF0000", "#0000FF").unwrap();
        let b = build_ani(Role::Arrow, "#FF0000", "#FF0000").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn crc_and_adler_match_known_values() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(Pattern::parse("wave").is_err());
        assert!(render(Role::Arrow, Pattern::Plain, "red", "#000000").is_err());
    }
}
