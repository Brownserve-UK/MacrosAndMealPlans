use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageFormat, RgbImage};

use super::process;

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
        width,
        height,
        image::Rgb([80, 40, 20]),
    ));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

fn jpeg_with_orientation(width: u32, height: u32, orientation: u16) -> Vec<u8> {
    let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(
        width,
        height,
        image::Rgb([80, 40, 20]),
    ));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
        .unwrap();
    let mut exif = vec![
        0xff, 0xe1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0, 0x2a, 0, 0, 0, 8, 0,
        1, 0x01, 0x12, 0, 3, 0, 0, 0, 1,
    ];
    exif.extend_from_slice(&orientation.to_be_bytes());
    exif.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    bytes.splice(2..2, exif);
    bytes
}

fn png_with_claimed_dimensions(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = png(1, 1);
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    let crc = crc32(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    bytes
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[test]
fn creates_jpeg_derivatives_without_upscaling() {
    let derivatives = process(&png(800, 400)).unwrap();

    assert_eq!(
        (derivatives.hero_width, derivatives.hero_height),
        (800, 400)
    );
    assert_eq!(
        (derivatives.card_width, derivatives.card_height),
        (640, 320)
    );
    assert!(derivatives.hero_jpeg.starts_with(&[0xff, 0xd8]));
    assert!(derivatives.card_jpeg.starts_with(&[0xff, 0xd8]));
}

#[test]
fn corrects_orientation_and_removes_metadata() {
    let derivatives = process(&jpeg_with_orientation(2, 3, 6)).unwrap();
    let decoded = image::load_from_memory(&derivatives.hero_jpeg).unwrap();

    assert_eq!(decoded.dimensions(), (3, 2));
    assert!(
        !derivatives
            .hero_jpeg
            .windows(6)
            .any(|window| window == b"Exif\0\0")
    );
}

#[test]
fn rejects_images_over_forty_megapixels_before_decoding() {
    assert_eq!(
        process(&png_with_claimed_dimensions(8_000, 5_001)).unwrap_err(),
        "That image is over 40 megapixels."
    );
}

#[test]
fn rejects_unsupported_and_animated_input() {
    assert!(process(b"not an image").is_err());

    let mut animated = png(2, 2);
    animated.splice(8..8, [0, 0, 0, 0, b'a', b'c', b'T', b'L', 0, 0, 0, 0]);
    assert_eq!(
        process(&animated).unwrap_err(),
        "Animated images are not supported."
    );
}
