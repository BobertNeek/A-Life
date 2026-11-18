//! A player's contact shelf is local and separate from every organism and world save.
//! Only explicitly exposed plates are written; a full shelf never destroys an original.
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub(super) const WIDTH: u32 = 640;
pub(super) const HEIGHT: u32 = 360;
const LIMIT: u64 = 2 * 1024 * 1024;
const SHELF: usize = 48;

fn plate_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix("qualia-")
        .and_then(|s| s.strip_suffix(".png"))
    else {
        return false;
    };
    let Some((time, seq)) = body.split_once('-') else {
        return false;
    };
    time.len() == 20
        && seq.len() == 3
        && [time, seq]
            .into_iter()
            .all(|s| s.bytes().all(|c| c.is_ascii_digit()))
}
fn valid_header(bytes: &[u8]) -> bool {
    bytes.get(..8) == Some(b"\x89PNG\r\n\x1a\n")
        && bytes.get(12..16) == Some(b"IHDR")
        && bytes.get(16..20) == Some(&WIDTH.to_be_bytes())
        && bytes.get(20..24) == Some(&HEIGHT.to_be_bytes())
}
pub(super) fn index(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut newest = std::collections::BTreeSet::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_name().to_str().is_some_and(plate_name) || !entry.file_type()?.is_file() {
            continue;
        }
        let mut header = [0; 24];
        if fs::File::open(entry.path())
            .and_then(|mut f| f.read_exact(&mut header))
            .is_err()
            || !valid_header(&header)
        {
            continue;
        }
        newest.insert(entry.path());
        if newest.len() > SHELF {
            newest.pop_first();
        }
    }
    Ok(newest.into_iter().rev().collect())
}
pub(super) fn store(folder: &Path, ms: u64, rgba: &image::RgbaImage) -> io::Result<PathBuf> {
    if rgba.dimensions() != (WIDTH, HEIGHT) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "wrong plate dimensions",
        ));
    }
    let mut png = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut png, image::ImageFormat::Png)
        .map_err(io::Error::other)?;
    fs::create_dir_all(folder)?;
    for n in 0..1000 {
        let path = folder.join(format!("qualia-{ms:020}-{n:03}.png"));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Err(error) = f.write_all(png.get_ref()).and_then(|()| f.sync_all()) {
                    drop(f);
                    let _ = fs::remove_file(&path); // Only our successful create_new owns this path.
                    return Err(error);
                }
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "plate exposure limit reached",
    ))
}
pub(super) fn read(path: &Path) -> io::Result<Vec<u8>> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "not a field-camera plate");
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > LIMIT {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT || !valid_header(&bytes) {
        return Err(invalid());
    }
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_fresh_shelf_can_reopen_exact_photos_without_replacing_colliding_exposures() {
        let folder = std::env::temp_dir().join(format!(
            "qualia-plate-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(index(&folder).unwrap().is_empty());
        let mut pixels =
            image::RgbaImage::from_pixel(WIDTH, HEIGHT, image::Rgba([93, 121, 140, 255]));
        let a = store(&folder, 143, &pixels).unwrap();
        pixels.put_pixel(7, 9, image::Rgba([222, 143, 77, 255]));
        let b = store(&folder, 143, &pixels).unwrap();
        assert_eq!(index(&folder).unwrap(), vec![b.clone(), a.clone()]);
        let decode = |p: &Path| {
            image::load_from_memory(&read(p).unwrap())
                .unwrap()
                .to_rgba8()
        };
        assert_ne!(decode(&a).get_pixel(7, 9), decode(&b).get_pixel(7, 9));
        fs::write(folder.join("unrelated.png"), b"leave me").unwrap();
        let broken = folder.join("qualia-00000000000000000201-000.png");
        fs::write(&broken, b"unfinished").unwrap();
        assert!(!index(&folder).unwrap().contains(&broken));
        for i in 144..=200 {
            fs::copy(&a, folder.join(format!("qualia-{i:020}-000.png"))).unwrap();
        }
        let visible = index(&folder).unwrap();
        assert_eq!(visible.len(), SHELF);
        assert!(a.exists() && b.exists());
        assert!(read(&broken).is_err());
        fs::remove_dir_all(&folder).unwrap();
    }
}
