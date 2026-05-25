use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use anyhow::{anyhow, Result};

use crate::app::paths::Paths;
use crate::model::config::Config;
use crate::model::state::State;

#[derive(Debug, Clone)]
pub struct Store {
    paths: Paths,
}

impl Store {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        ensure_private_dir(&self.paths.config_dir)?;
        let serialized = toml::to_string_pretty(config)?;
        atomic_write(&self.paths.config_file, serialized.as_bytes())?;
        Ok(())
    }

    pub fn load_config(&self) -> Result<Config> {
        let raw = fs::read_to_string(&self.paths.config_file)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save_state(&self, state: &State) -> Result<()> {
        ensure_private_dir(&self.paths.state_dir)?;
        let serialized = serde_json::to_string_pretty(state)?;
        atomic_write(&self.paths.state_file, serialized.as_bytes())?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<State> {
        let raw = fs::read_to_string(&self.paths.state_file)?;
        Ok(serde_json::from_str(&raw)?)
    }
}

fn ensure_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("path has no parent directory: {}", path.display()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("path has no file name: {}", path.display()))?
        .to_string_lossy();

    for attempt in 0..16usize {
        let epoch_nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let candidate = parent.join(format!(
            ".{file_name}.tmp-{}-{epoch_nanos}-{attempt}",
            process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);

        match options.open(&candidate) {
            Ok(mut temp_file) => {
                if let Err(write_error) = temp_file
                    .write_all(contents)
                    .and_then(|()| temp_file.sync_all())
                {
                    drop(temp_file);
                    let _ = fs::remove_file(&candidate);
                    return Err(write_error.into());
                }
                drop(temp_file);
                if let Err(rename_error) = fs::rename(&candidate, path) {
                    let _ = fs::remove_file(&candidate);
                    return Err(rename_error.into());
                }
                set_private_file_permissions(path)?;
                return Ok(());
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Err(anyhow!(
        "failed to allocate temp file for atomic write: {}",
        path.display()
    ))
}
