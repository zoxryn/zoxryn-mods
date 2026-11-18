//! Bounded raster decoding; no original file paths or metadata are retained.
use crate::safety::*;
use anyhow::{bail, Context, Result};
use image::{ImageFormat, ImageReader, Limits};
use std::{
    fs,
    io::{Cursor, Read},
    path::Path,
};
pub const MAX_IMAGE: u64 = 8 * 1024 * 1024;
pub const MAX_THUMBNAIL: usize = 640 * 1024;
pub fn normalize(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() as u64 > MAX_IMAGE {
        bail!("Preview exceeds 8 MiB");
    }
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader.format().context("Unrecognized preview image")?;
    if ![ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP].contains(&format) {
        bail!("Use PNG, JPEG or WebP artwork; SVG and executable formats are not supported");
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    let mut header = ImageReader::with_format(Cursor::new(bytes), format);
    header.limits(limits.clone());
    let (w, h) = header.into_dimensions()?;
    if w == 0 || h == 0 || w > 4096 || h > 4096 || u64::from(w) * u64::from(h) > 4_194_304 {
        bail!("Preview must be at most 4096 pixels per side and 4 megapixels");
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let image = reader.decode()?.thumbnail(384, 384);
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, ImageFormat::Png)?;
    if output.get_ref().len() > MAX_THUMBNAIL {
        bail!("Normalized preview exceeds the thumbnail budget");
    }
    Ok(output.into_inner())
}
pub fn store(data: &Path, bytes: &[u8]) -> Result<String> {
    let thumbnail = normalize(bytes)?;
    let key = hash_bytes(&thumbnail);
    let dir = data.join("thumbnails");
    let _guard = guard_tree(&dir, true)?;
    let path = dir.join(format!("{key}.png"));
    if path.exists() {
        if hash_file(&path)? != key {
            bail!("Cached preview was modified");
        }
    } else {
        let tmp = dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&tmp, thumbnail)?;
        fs::File::options().write(true).open(&tmp)?.sync_all()?;
        fs::rename(tmp, path)?;
    }
    Ok(key)
}
pub fn import(data: &Path, path: &Path) -> Result<String> {
    let mut file = open_read_no_follow(path)?;
    if file.metadata()?.len() > MAX_IMAGE {
        bail!("Preview exceeds 8 MiB");
    }
    let mut bytes = Vec::new();
    file.by_ref().take(MAX_IMAGE + 1).read_to_end(&mut bytes)?;
    store(data, &bytes)
}
pub fn read(data: &Path, key: &str) -> Result<Vec<u8>> {
    if key.len() != 64 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Invalid thumbnail key");
    }
    let mut file = open_read_no_follow(&data.join("thumbnails").join(format!("{key}.png")))?;
    if file.metadata()?.len() > MAX_THUMBNAIL as u64 {
        bail!("Cached thumbnail is oversized");
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_THUMBNAIL as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_THUMBNAIL || hash_bytes(&bytes) != key {
        bail!("Cached thumbnail failed integrity verification");
    }
    Ok(bytes)
}
