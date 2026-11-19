//! Подготовка значка к показу: обрезка пустых полей от «растянутого» значка, плавное
//! уменьшение до размера плитки x2 (HiDPI) без тёмной каймы по краю прозрачности и PNG.
//! Чистая обработка пикселей, без обращений к системе, поэтому проверяется на любой ОС.

use std::io::Cursor;

use image::imageops::{self, FilterType};
use image::{ImageFormat, Rgba, RgbaImage};

/// Пиксель считается «содержимым», если непрозрачнее этого порога.
const ALPHA_THRESHOLD: u8 = 8;
/// Значок меньше этой доли холста считается «вклеенным в большой холст» и обрезается.
const PADDED_FRACTION_PERCENT: u32 = 55;

/// Рамка непрозрачных пикселей: `(left, top, right, bottom)`, правая и нижняя граница не включены.
fn content_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0[3] > ALPHA_THRESHOLD {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    (right > left && bottom > top).then_some((left, top, right, bottom))
}

/// Квадратный холст со значком по центру. Лишние поля убираются, только если значок занимает
/// малую часть холста (оболочка кладёт значок 48 px в центр 256 px).
fn squared(image: &RgbaImage) -> Option<RgbaImage> {
    let (left, top, right, bottom) = content_bounds(image)?;
    let (width, height) = (right - left, bottom - top);
    let canvas = image.width().max(image.height());
    let padded = width.max(height) * 100 < canvas * PADDED_FRACTION_PERCENT;
    let (source, sw, sh) = if padded {
        (
            imageops::crop_imm(image, left, top, width, height).to_image(),
            width,
            height,
        )
    } else {
        (image.clone(), image.width(), image.height())
    };
    let side = sw.max(sh);
    if sw == sh {
        return Some(source);
    }
    let mut out = RgbaImage::from_pixel(side, side, Rgba([0, 0, 0, 0]));
    imageops::replace(
        &mut out,
        &source,
        i64::from((side - sw) / 2),
        i64::from((side - sh) / 2),
    );
    Some(out)
}

fn premultiply(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let a = u32::from(pixel.0[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * a + 127) / 255) as u8;
        }
    }
}

fn unpremultiply(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let a = u32::from(pixel.0[3]);
        if a == 0 {
            continue;
        }
        for channel in &mut pixel.0[..3] {
            *channel = ((u32::from(*channel) * 255 + a / 2) / a).min(255) as u8;
        }
    }
}

/// Значок как квадрат `px` x `px`. `None` — значок пустой (полностью прозрачный).
pub fn fit_icon(image: &RgbaImage, px: u32) -> Option<RgbaImage> {
    let mut square = squared(image)?;
    if square.width() == px {
        return Some(square);
    }
    premultiply(&mut square);
    let mut resized = imageops::resize(&square, px, px, FilterType::Lanczos3);
    unpremultiply(&mut resized);
    Some(resized)
}

/// PNG-байты значка `px` x `px`. `None` — значок пустой или кодирование не удалось.
pub fn icon_png(image: &RgbaImage, px: u32) -> Option<Vec<u8>> {
    let fitted = fit_icon(image, px)?;
    let mut bytes = Cursor::new(Vec::new());
    fitted.write_to(&mut bytes, ImageFormat::Png).ok()?;
    Some(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(color))
    }

    #[test]
    fn full_icon_is_scaled_to_the_requested_square() {
        let out = fit_icon(&solid(256, 256, [200, 30, 30, 255]), 64).unwrap();
        assert_eq!((out.width(), out.height()), (64, 64));
        assert_eq!(out.get_pixel(32, 32).0, [200, 30, 30, 255]);
    }

    #[test]
    fn small_icon_pasted_into_a_big_canvas_is_cropped_and_enlarged() {
        let mut canvas = solid(256, 256, [0, 0, 0, 0]);
        for y in 104..152 {
            for x in 104..152 {
                canvas.put_pixel(x, y, Rgba([10, 200, 10, 255]));
            }
        }
        let out = fit_icon(&canvas, 64).unwrap();
        assert_eq!(
            out.get_pixel(1, 1).0,
            [10, 200, 10, 255],
            "fills the square"
        );
        assert_eq!(out.get_pixel(62, 62).0, [10, 200, 10, 255]);
    }

    #[test]
    fn icon_that_fills_its_canvas_keeps_its_margins() {
        let mut canvas = solid(256, 256, [0, 0, 0, 0]);
        for y in 16..240 {
            for x in 16..240 {
                canvas.put_pixel(x, y, Rgba([9, 9, 200, 255]));
            }
        }
        let out = fit_icon(&canvas, 64).unwrap();
        assert_eq!(out.get_pixel(0, 0).0[3], 0, "margin stays transparent");
        assert_eq!(out.get_pixel(32, 32).0, [9, 9, 200, 255]);
    }

    #[test]
    fn non_square_content_is_centred_in_a_square() {
        let mut canvas = solid(256, 256, [0, 0, 0, 0]);
        for y in 100..156 {
            for x in 40..216 {
                canvas.put_pixel(x, y, Rgba([200, 200, 0, 255]));
            }
        }
        let out = fit_icon(&canvas, 64).unwrap();
        assert!(out.get_pixel(32, 32).0[3] > 250);
        assert_eq!(out.get_pixel(32, 0).0[3], 0, "bars above and below");
    }

    #[test]
    fn transparent_edge_does_not_get_a_dark_halo() {
        let mut image = solid(128, 128, [0, 0, 0, 0]);
        for y in 0..128 {
            for x in 0..64 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let out = fit_icon(&image, 32).unwrap();
        for pixel in out.pixels().filter(|p| p.0[3] > 20) {
            assert!(pixel.0[0] >= 245, "white stays white: {:?}", pixel.0);
        }
    }

    #[test]
    fn fully_transparent_icon_is_empty() {
        assert!(fit_icon(&solid(48, 48, [255, 0, 0, 0]), 64).is_none());
        assert!(icon_png(&solid(48, 48, [0, 0, 0, 0]), 64).is_none());
    }

    #[test]
    fn png_bytes_have_the_png_signature_and_decode_to_the_size() {
        let bytes = icon_png(&solid(96, 96, [1, 2, 3, 255]), 64).unwrap();
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 64));
    }
}
