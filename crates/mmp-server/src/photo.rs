use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader, Limits};
use mmp_core::domain::RecipePhotoDerivatives;

const MAX_PIXELS: u64 = 40_000_000;
const MAX_ALLOCATION: u64 = 256 * 1024 * 1024;

pub fn process(bytes: &[u8]) -> Result<RecipePhotoDerivatives, String> {
    let format =
        image::guess_format(bytes).map_err(|_| "Choose a JPEG, PNG, or WebP image.".to_owned())?;
    if !matches!(
        format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP
    ) {
        return Err("Choose a JPEG, PNG, or WebP image.".to_owned());
    }
    if is_animated(bytes, format) {
        return Err("Animated images are not supported.".to_owned());
    }

    let reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| "That image could not be read.".to_owned())?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("That image is over 40 megapixels.".to_owned());
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(MAX_ALLOCATION);
    decoder
        .set_limits(limits)
        .map_err(|_| "That image is too large to process.".to_owned())?;
    let orientation = decoder
        .orientation()
        .map_err(|_| "That image could not be read.".to_owned())?;
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|_| "That image could not be read.".to_owned())?;
    image.apply_orientation(orientation);

    let hero = resize(&image, 1_920);
    let card = resize(&image, 640);
    let (hero_width, hero_height) = hero.dimensions();
    let (card_width, card_height) = card.dimensions();
    Ok(RecipePhotoDerivatives {
        hero_jpeg: encode_jpeg(&hero, 85)?,
        card_jpeg: encode_jpeg(&card, 82)?,
        hero_width: hero_width as i32,
        hero_height: hero_height as i32,
        card_width: card_width as i32,
        card_height: card_height as i32,
    })
}

fn resize(image: &DynamicImage, longest_edge: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width <= longest_edge && height <= longest_edge {
        return image.clone();
    }
    image.resize(longest_edge, longest_edge, FilterType::Lanczos3)
}

fn encode_jpeg(image: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode_image(image)
        .map_err(|_| "That image could not be processed.".to_owned())?;
    Ok(bytes)
}

fn is_animated(bytes: &[u8], format: ImageFormat) -> bool {
    match format {
        ImageFormat::Png => png_is_animated(bytes),
        ImageFormat::WebP => webp_is_animated(bytes),
        _ => false,
    }
}

fn png_is_animated(bytes: &[u8]) -> bool {
    let mut offset = 8;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        if kind == b"acTL" {
            return true;
        }
        let Some(next) = offset.checked_add(12 + length) else {
            return false;
        };
        if next > bytes.len() || kind == b"IEND" {
            return false;
        }
        offset = next;
    }
    false
}

fn webp_is_animated(bytes: &[u8]) -> bool {
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let kind = &bytes[offset..offset + 4];
        if kind == b"ANIM" || kind == b"ANMF" {
            return true;
        }
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let Some(next) = offset.checked_add(8 + length + (length % 2)) else {
            return false;
        };
        if next > bytes.len() {
            return false;
        }
        offset = next;
    }
    false
}

#[cfg(test)]
#[path = "photo_tests.rs"]
mod tests;
