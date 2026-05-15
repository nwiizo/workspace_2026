//! JSON snapshot persistence with atomic rename.
//!
//! Each service owns one logical snapshot named `{service_name}.json` under the
//! configured data directory. Writes go to a tempfile in the same directory and
//! are renamed into place to give us crash-consistent atomicity on POSIX
//! filesystems.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("io error on {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid snapshot name {0:?} — must match [a-z0-9_-]+")]
    InvalidName(String),
}

fn validate_name(name: &str) -> Result<(), PersistError> {
    // Path-traversal guard. Snapshot names today are always &'static service
    // identifiers, but defending in depth protects future export endpoints
    // that might forward user-controlled names into Snapshot::save/load.
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err(PersistError::InvalidName(name.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    base: PathBuf,
}

impl Snapshot {
    /// Create a snapshot store rooted at `base`. The directory is created lazily
    /// on the first write so an unset KUROKO_DATA_DIR keeps the store inert.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Load `{name}.json` and deserialize. Returns `Ok(None)` when the file does
    /// not exist — callers treat that as "empty initial state".
    pub fn load<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, PersistError> {
        validate_name(name)?;
        let path = self.path_for(name);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(PersistError::Io { path, source: e }),
        };
        let value = serde_json::from_slice(&bytes)?;
        Ok(Some(value))
    }

    /// Atomically write `{name}.json` containing the JSON representation of `value`.
    pub fn save<T: Serialize>(&self, name: &str, value: &T) -> Result<(), PersistError> {
        validate_name(name)?;
        fs::create_dir_all(&self.base).map_err(|e| PersistError::Io {
            path: self.base.clone(),
            source: e,
        })?;

        let final_path = self.path_for(name);
        let tmp_path = final_path.with_extension("json.tmp");

        {
            let mut file = fs::File::create(&tmp_path).map_err(|e| PersistError::Io {
                path: tmp_path.clone(),
                source: e,
            })?;
            let bytes = serde_json::to_vec(value)?;
            file.write_all(&bytes).map_err(|e| PersistError::Io {
                path: tmp_path.clone(),
                source: e,
            })?;
            file.sync_all().map_err(|e| PersistError::Io {
                path: tmp_path.clone(),
                source: e,
            })?;
        }

        fs::rename(&tmp_path, &final_path).map_err(|e| PersistError::Io {
            path: final_path,
            source: e,
        })?;
        Ok(())
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.base.join(format!("{name}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct State {
        items: Vec<String>,
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let snap = Snapshot::new(dir.path());
        let state = State {
            items: vec!["a".into(), "b".into()],
        };
        snap.save("svc", &state).unwrap();
        let loaded: State = snap.load("svc").unwrap().unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn load_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let snap = Snapshot::new(dir.path());
        let loaded: Option<State> = snap.load("nope").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn rejects_path_traversal_names() {
        let dir = tempfile::tempdir().unwrap();
        let snap = Snapshot::new(dir.path());
        let s = State { items: vec![] };
        for bad in ["..", "../etc/passwd", "a/b", "S3", "x.y", ""] {
            assert!(matches!(
                snap.save(bad, &s),
                Err(PersistError::InvalidName(_))
            ));
            assert!(matches!(
                snap.load::<State>(bad),
                Err(PersistError::InvalidName(_))
            ));
        }
    }
}
