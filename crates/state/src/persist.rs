use error::{YError, YResult, log_to_file};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::PathBuf};

pub trait Persist: Sized + Default + Serialize + DeserializeOwned {
    const FILE_NAME: &'static str;

    fn get_path() -> YResult<PathBuf> {
        dirs::state_dir()
            .map(|p| p.join(format!("gytm/{}", Self::FILE_NAME)))
            .ok_or(YError::InvalidPath("STATE_DIR".to_string()))
    }

    fn load() -> YResult<Self> {
        let path = Self::get_path()?;
        if path.exists() && path.is_file() {
            let content = fs::read_to_string(&path)?;
            match serde_json::from_str(&content) {
                Ok(state) => return Ok(state),
                Err(e) => log_to_file(&e),
            }
        }
        Self::create()
    }

    fn create() -> YResult<Self> {
        let path = Self::get_path()?;
        fs::create_dir_all(
            path.parent()
                .ok_or(YError::InvalidPath("STATE_DIR".to_string()))?,
        )?;
        let def = Self::default();
        fs::write(&path, serde_json::to_string(&def)?)?;
        Ok(def)
    }

    fn save(&self) -> YResult<()> {
        let path = Self::get_path()?;
        let f = fs::File::create(&path)?;
        serde_json::to_writer(f, self)?;
        Ok(())
    }
}