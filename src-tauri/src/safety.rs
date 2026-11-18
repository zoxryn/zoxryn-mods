use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub const MAX_ARCHIVE: u64 = 1024 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_FILE: u64 = 256 * 1024 * 1024;

pub fn open_read_no_follow(path: &Path) -> Result<File> {
    check_tree(path)?;
    let mut options = File::options();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000).share_mode(0x1);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if is_link(&metadata) || !metadata.is_file() {
        bail!("Not a regular, non-reparse file: {}", path.display());
    }
    Ok(file)
}
pub fn link_count(file: &File) -> Result<u32> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        #[repr(C)]
        struct Info {
            attrs: u32,
            creation: [u32; 2],
            access: [u32; 2],
            write: [u32; 2],
            volume: u32,
            size_high: u32,
            size_low: u32,
            links: u32,
            index_high: u32,
            index_low: u32,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut Info) -> i32;
        }
        let mut info: Info = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(info.links)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(file.metadata()?.nlink() as u32)
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = file;
        Ok(1)
    }
}

/// Keep ancestor directories stable while using approved path-based operations.
/// On Windows, deny write/delete sharing on directory handles and open each
/// component as a reparse point before checking its actual handle metadata.
/// Parents are held before a child is created or opened.
pub fn guard_tree(directory: &Path, create_missing: bool) -> Result<Vec<File>> {
    guard_directories(&[directory.to_path_buf()], create_missing)
}
pub fn guard_existing_tree(directory: &Path) -> Result<Vec<File>> {
    let guards = guard_tree(directory, false)?;
    #[cfg(windows)]
    if guards.len() != directory.ancestors().count() {
        bail!(
            "Directory disappeared before it could be held stable: {}",
            directory.display()
        );
    }
    check_tree(directory)?;
    if !directory.is_dir() {
        bail!("Directory is unavailable: {}", directory.display());
    }
    Ok(guards)
}
pub fn guard_directories(directories: &[PathBuf], create_missing: bool) -> Result<Vec<File>> {
    let unique: std::collections::HashSet<PathBuf> = directories
        .iter()
        .flat_map(|d| d.ancestors().map(Path::to_path_buf))
        .collect();
    let mut ancestors: Vec<_> = unique.into_iter().collect();
    ancestors.sort_by_key(|p| (p.components().count(), p.clone()));
    let mut handles = Vec::new();
    for path in ancestors {
        if !path.exists() {
            if !create_missing {
                continue;
            }
            match fs::create_dir(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let handle = File::options().access_mode(0x80).share_mode(0x1).custom_flags(0x02000000 | 0x00200000).open(&path)
                .with_context(||format!("Could not hold directory stable: {}. Close programs modifying this folder.",path.display()))?;
            let meta = handle.metadata()?;
            if is_link(&meta) || !meta.is_dir() {
                bail!(
                    "Reparse point or non-directory component: {}",
                    path.display()
                );
            }
            handles.push(handle);
        }
        #[cfg(not(windows))]
        {
            let meta = fs::symlink_metadata(&path)?;
            if is_link(&meta) || !meta.is_dir() {
                bail!("Symlink or non-directory component: {}", path.display());
            }
        }
    }
    Ok(handles)
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// Windows paths are checked even when tests run on another platform.
pub fn relative_name(name: &str) -> Result<String> {
    let n = name.replace('\\', "/");
    if n.is_empty() || n.starts_with('/') || n.len() > 220 || n.contains(':') || n.contains('\0') {
        bail!("Unsafe archive path: {name}");
    }
    for part in n.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with([' ', '.'])
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
        {
            bail!("Unsafe archive path: {name}");
        }
        let stem = part.split('.').next().unwrap_or("").to_uppercase();
        if ["CON", "PRN", "AUX", "NUL", "CLOCK$"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'0'..=b'9'))
        {
            bail!("Reserved Windows path: {name}");
        }
    }
    Ok(n)
}
pub fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
pub fn check_tree(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(ancestor) {
            if is_link(&meta) {
                bail!(
                    "Symlink or reparse point is not allowed: {}",
                    ancestor.display()
                );
            }
        }
    }
    Ok(())
}
pub fn target(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = relative_name(rel)?;
    let path = root.join(rel);
    check_tree(&path)?;
    if !path.starts_with(root) {
        bail!("Target escaped approved game folder");
    }
    if path.exists() && !path.is_file() {
        bail!("Target is not a regular file: {}", path.display());
    }
    Ok(path)
}
pub fn current_hash(path: &Path) -> Result<Option<String>> {
    if path.exists() {
        Ok(Some(hash_file(path)?))
    } else {
        Ok(None)
    }
}
pub fn copy_verified(from: &Path, to: &Path, expected: &str) -> Result<()> {
    check_tree(from)?;
    check_tree(to)?;
    if hash_file(from)? != expected {
        bail!("Stored file changed: {}", from.display());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
        check_tree(parent)?;
    }
    fs::copy(from, to).with_context(|| format!("Copy failed: {}", to.display()))?;
    File::options().write(true).open(to)?.sync_all()?;
    if hash_file(to)? != expected {
        bail!("Verification failed: {}", to.display());
    }
    Ok(())
}
