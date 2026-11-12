//! Validates background preferences and stores immutable images separately from settings metadata.

use std::{
    fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

use nsh_core::image::Natural;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackgroundConfig {
    pub(crate) default_opacity: u32,
    pub(crate) min_opacity: u32,
    pub(crate) max_opacity: u32,
    pub(crate) content_transparency: ContentTransparencyConfig,
    pub(crate) max_image_bytes: usize,
    pub(crate) max_image_pixels: u64,
    resource_directory: String,
    asset_prefix: String,
}

#[derive(Deserialize)]
pub(crate) struct ContentTransparencyConfig {
    pub(crate) default: u32,
    pub(crate) min: u32,
    pub(crate) max: u32,
}

pub(crate) static CONFIG: LazyLock<BackgroundConfig> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../ui/config/background.json"))
        .expect("the bundled background configuration is valid JSON")
});

/// Metadata in settings; the ID is an application-owned basename, never a user path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct BackgroundImage {
    pub(crate) id: String,
    pub(crate) name: String,
}

impl BackgroundImage {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let mut parts = Path::new(&self.id).components();
        if !self.id.starts_with(&CONFIG.asset_prefix)
            || !matches!(parts.next(), Some(Component::Normal(_)))
            || parts.next().is_some()
            || self.id.contains(['/', '\\'])
            || self.name.is_empty()
            || self.name.chars().any(char::is_control)
        {
            return Err("background_image_invalid");
        }
        Ok(())
    }
}

pub(crate) fn default_opacity() -> u32 {
    CONFIG.default_opacity
}
pub(crate) fn is_default_opacity(value: &u32) -> bool {
    *value == default_opacity()
}
pub(crate) fn validate_opacity(value: u32) -> Result<(), &'static str> {
    if (CONFIG.min_opacity..=CONFIG.max_opacity).contains(&value) {
        Ok(())
    } else {
        Err("background_opacity_invalid")
    }
}

pub(crate) fn default_content_transparency() -> u32 {
    CONFIG.content_transparency.default
}

pub(crate) fn is_default_content_transparency(value: &u32) -> bool {
    *value == default_content_transparency()
}

pub(crate) fn validate_content_transparency(value: u32) -> Result<(), &'static str> {
    if (CONFIG.content_transparency.min..=CONFIG.content_transparency.max).contains(&value) {
        Ok(())
    } else {
        Err("content_background_transparency_invalid")
    }
}

/// The current import stays cached for session-only hosts and failed settings writes.
pub(crate) struct BackgroundAssets {
    directory: Option<PathBuf>,
    current: Option<(BackgroundImage, Vec<u8>)>,
    session_id: u64,
}

impl BackgroundAssets {
    pub(crate) fn new(settings_directory: Option<&Path>) -> Self {
        Self {
            directory: settings_directory.map(|root| root.join(&CONFIG.resource_directory)),
            current: None,
            session_id: 0,
        }
    }

    /// Publish the complete image before settings can reference it. Retrying the same import
    /// reuses its ID so a failed metadata write cannot create duplicate assets.
    pub(crate) fn import(
        &mut self,
        name: String,
        bytes: Vec<u8>,
        persistent: bool,
    ) -> Result<BackgroundImage, &'static str> {
        validate_bytes(&bytes)?;
        let name = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_owned();
        if let Some((image, cached)) = &self.current
            && image.name == name
            && *cached == bytes
        {
            return Ok(image.clone());
        }
        let id = if let Some(directory) = self.directory.as_deref().filter(|_| persistent) {
            ensure_directory(directory).map_err(|_| "background_image_write_failed")?;
            let mut file = tempfile::Builder::new()
                .prefix(&CONFIG.asset_prefix)
                .tempfile_in(directory)
                .map_err(|_| "background_image_write_failed")?;
            file.write_all(&bytes)
                .and_then(|()| file.as_file().sync_all())
                .map_err(|_| "background_image_write_failed")?;
            let id = file
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("background_image_write_failed")?
                .to_owned();
            let image = BackgroundImage {
                id: id.clone(),
                name: name.clone(),
            };
            image.validate()?;
            file.keep().map_err(|_| "background_image_write_failed")?;
            id
        } else {
            self.session_id += 1;
            format!("{}session-{}", CONFIG.asset_prefix, self.session_id)
        };
        let image = BackgroundImage { id, name };
        image.validate()?;
        self.current = Some((image.clone(), bytes));
        Ok(image)
    }

    pub(crate) fn load(&self, image: &BackgroundImage) -> Result<Vec<u8>, &'static str> {
        image.validate()?;
        if let Some((cached, bytes)) = &self.current
            && cached.id == image.id
        {
            return Ok(bytes.clone());
        }
        let directory = self
            .directory
            .as_deref()
            .ok_or("background_image_unavailable")?;
        if !fs::symlink_metadata(directory).is_ok_and(|metadata| metadata.is_dir()) {
            return Err("background_image_unavailable");
        }
        let path = directory.join(&image.id);
        let metadata = fs::symlink_metadata(&path).map_err(|_| "background_image_unavailable")?;
        if !metadata.is_file() || metadata.len() > CONFIG.max_image_bytes as u64 {
            return Err("background_image_unavailable");
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .and_then(|file| {
                file.take(CONFIG.max_image_bytes as u64 + 1)
                    .read_to_end(&mut bytes)
            })
            .map_err(|_| "background_image_unavailable")?;
        validate_bytes(&bytes)?;
        Ok(bytes)
    }

    pub(crate) fn retain_cached(&mut self, selected: Option<&BackgroundImage>) {
        if self
            .current
            .as_ref()
            .is_some_and(|(image, _)| Some(image) != selected)
        {
            self.current = None;
        }
    }

    /// Only run after the settings commit succeeds, so the last saved image survives any
    /// failed replacement or clear. Unrelated files and directories are left alone.
    pub(crate) fn prune(&mut self, selected: Option<&BackgroundImage>) {
        self.retain_cached(selected);
        let Some(directory) = self.directory.as_deref() else {
            return;
        };
        if !fs::symlink_metadata(directory).is_ok_and(|metadata| metadata.is_dir()) {
            return;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(id) = name.to_str() else {
                continue;
            };
            if id.starts_with(&CONFIG.asset_prefix)
                && selected.is_none_or(|image| image.id != id)
                && entry.file_type().is_ok_and(|kind| kind.is_file())
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn validate_bytes(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() > CONFIG.max_image_bytes {
        return Err("background_image_too_large");
    }
    match Natural::of(bytes) {
        Some(Natural::Pixels(width, height))
            if u64::from(width) * u64::from(height) <= CONFIG.max_image_pixels =>
        {
            Ok(())
        }
        _ => Err("background_image_invalid"),
    }
}

fn ensure_directory(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    if fs::symlink_metadata(directory)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "background directory is a symlink",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png() -> Vec<u8> {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/background.json")).unwrap();
        nsh_core::image::decode_data_uri(fixture["dataUrl"].as_str().unwrap())
            .unwrap()
            .1
    }

    #[test]
    fn imports_are_immutable_and_retry_reuses_the_id() {
        let root = tempfile::tempdir().unwrap();
        let mut assets = BackgroundAssets::new(Some(root.path()));
        let first = assets.import("first.png".into(), png(), true).unwrap();
        assert_eq!(
            assets.import("first.png".into(), png(), true).unwrap(),
            first
        );
        let second = assets.import("second.png".into(), png(), true).unwrap();
        let reopened = BackgroundAssets::new(Some(root.path()));
        assert_eq!(reopened.load(&first).unwrap(), png());
        assert_eq!(reopened.load(&second).unwrap(), png());
        assets.prune(Some(&second));
        assert!(reopened.load(&first).is_err());
        assert_eq!(reopened.load(&second).unwrap(), png());
    }

    #[test]
    fn rejects_invalid_images_and_paths_without_replacing_the_selection() {
        let mut assets = BackgroundAssets::new(None);
        let image = assets.import("valid.png".into(), png(), false).unwrap();
        assert_eq!(
            assets.import("invalid.png".into(), b"not an image".to_vec(), false),
            Err("background_image_invalid")
        );
        assert_eq!(assets.load(&image).unwrap(), png());
        assert!(
            BackgroundImage {
                id: "image-../../settings.toml".into(),
                name: "bad".into()
            }
            .validate()
            .is_err()
        );
        assert!(validate_bytes(&vec![0; CONFIG.max_image_bytes + 1]).is_err());
    }
}
