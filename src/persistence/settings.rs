//! Reads and atomically replaces settings bytes without owning business state.
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use tempfile::Builder;
pub(crate) fn read(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}
pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = parent_dir(path);
    fs::create_dir_all(directory)?;
    let mut temporary = Builder::new().prefix(".settings-").tempfile_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
pub(crate) fn preserve_invalid(path: &Path) -> io::Result<()> {
    let (placeholder, backup_path) = Builder::new()
        .prefix("settings.toml.corrupt-")
        .tempfile_in(parent_dir(path))?
        .keep()
        .map_err(|error| error.error)?;
    drop(placeholder);

    fs::rename(path, &backup_path).inspect_err(|_| {
        let _ = fs::remove_file(&backup_path);
    })
}

pub(crate) fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
