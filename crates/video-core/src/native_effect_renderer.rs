//! Deterministic native raster previews for effect-registry fixtures.
//!
//! This deliberately has no browser, Node, FFmpeg, or external renderer
//! dependency. It turns a validated native effect description into a PNG
//! using only Rust stdlib so preview pixels remain reproducible.

use std::fs;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NativeEffectRenderError {
    #[error("native effect preview dimensions must be nonzero")]
    ZeroDimensions,
    #[error("native effect preview I/O: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct NativeEffectFrame {
    pub width: u32,
    pub height: u32,
    pub footprint_px: (u32, u32, u32, u32),
    pub accent_rgb: (u8, u8, u8),
    pub animated: bool,
    pub reduced_motion: bool,
}

/// Render one native effect preview frame as a valid, deterministic PNG.
/// Animated frames receive a bounded inset accent; reduced-motion frames
/// keep the settled geometry from frame zero.
pub fn render_native_effect_frame(
    frame: &NativeEffectFrame,
    output: &Path,
) -> Result<(), NativeEffectRenderError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(NativeEffectRenderError::ZeroDimensions);
    }
    let mut pixels = vec![0_u8; frame.width as usize * frame.height as usize * 3];
    for y in 0..frame.height {
        for x in 0..frame.width {
            let offset = ((y * frame.width + x) * 3) as usize;
            let shade = 12 + ((x + y) % 11) as u8;
            pixels[offset..offset + 3].copy_from_slice(&[shade, shade, shade + 4]);
        }
    }
    let (x, y, width, height) = frame.footprint_px;
    let inset = if frame.animated && !frame.reduced_motion {
        12
    } else {
        0
    };
    let x0 = x.saturating_add(inset).min(frame.width);
    let y0 = y.saturating_add(inset).min(frame.height);
    let x1 = x
        .saturating_add(width)
        .saturating_sub(inset)
        .min(frame.width);
    let y1 = y
        .saturating_add(height)
        .saturating_sub(inset)
        .min(frame.height);
    for py in y0..y1 {
        for px in x0..x1 {
            let offset = ((py * frame.width + px) * 3) as usize;
            let edge = (px - x0)
                .min(x1.saturating_sub(px))
                .min((py - y0).min(y1.saturating_sub(py)));
            let blend = if edge < 6 {
                255
            } else {
                64 + (((px - x0) + (py - y0)) % 128) as u8
            };
            let color = [
                ((frame.accent_rgb.0 as u16 * blend as u16 + 20 * (255 - blend as u16)) / 255)
                    as u8,
                ((frame.accent_rgb.1 as u16 * blend as u16 + 20 * (255 - blend as u16)) / 255)
                    as u8,
                ((frame.accent_rgb.2 as u16 * blend as u16 + 22 * (255 - blend as u16)) / 255)
                    as u8,
            ];
            pixels[offset..offset + 3].copy_from_slice(&color);
        }
    }
    fs::write(output, png_rgb(frame.width, frame.height, &pixels))?;
    Ok(())
}

fn png_rgb(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let stride = width as usize * 3;
    let mut scanlines = Vec::with_capacity((stride + 1) * height as usize);
    for row in pixels.chunks_exact(stride) {
        scanlines.push(0);
        scanlines.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    for (index, chunk) in scanlines.chunks(65_535).enumerate() {
        zlib.push(if index + 1 == scanlines.chunks(65_535).len() {
            1
        } else {
            0
        });
        let len = chunk.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    zlib.extend_from_slice(&adler32(&scanlines).to_be_bytes());

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &zlib);
    png_chunk(&mut png, b"IEND", &[]);
    png
}

fn png_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut crc_bytes = kind.to_vec();
    crc_bytes.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_bytes).to_be_bytes());
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in bytes {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_a_valid_png_signature() {
        let path = std::env::temp_dir().join("cutright-native-effect-preview.png");
        render_native_effect_frame(
            &NativeEffectFrame {
                width: 8,
                height: 8,
                footprint_px: (1, 1, 6, 6),
                accent_rgb: (1, 2, 3),
                animated: true,
                reduced_motion: false,
            },
            &path,
        )
        .unwrap();
        assert_eq!(&fs::read(&path).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
        let _ = fs::remove_file(path);
    }
}
