use error::{YError, YResult, log_to_file};
use serde::{Serialize, de::DeserializeOwned};
use std::{fs, path::PathBuf};

pub trait Persist: Sized + Default + Serialize + DeserializeOwned {
    const FILE_NAME: &'static str;

    fn get_path() -> YResult<PathBuf> {
        dirs::state_dir()
            .map(|p| p.join("gytm").join(Self::FILE_NAME))
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
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let tmp_path = path.with_extension("json.tmp");
        {
            let f = fs::File::create(&tmp_path)?;
            let writer = std::io::BufWriter::new(f);
            serde_json::to_writer(writer, self)?;
        }
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    fn delete() -> YResult<()> {
        let path = Self::get_path()?;
        fs::remove_file(path)?;
        Ok(())
    }
}
