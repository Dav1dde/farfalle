use axum::Extension;
use std::{fmt, io, ops::Deref, path::PathBuf, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::{fs::File, io::AsyncRead};

use crate::{IdGen, StorageExtension};

#[derive(thiserror::Error, Debug)]
#[error("")]
pub struct SaveError;

#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("unknown paste id")]
    NotFound,

    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug)]
pub struct PasteId(String);

impl PasteId {
    pub fn new(id: String) -> Result<Self, String> {
        if id
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9'))
        {
            Ok(Self(id))
        } else {
            Err(id)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for PasteId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for PasteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<PasteId> for String {
    fn from(id: PasteId) -> Self {
        id.0
    }
}

impl<'de> serde::de::Deserialize<'de> for PasteId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        let id = String::deserialize(deserializer)?;

        PasteId::new(id)
            .map_err(|id| Error::invalid_value(Unexpected::Str(&id), &"a valid paste id"))
    }
}

#[async_trait::async_trait]
pub trait Storage {
    async fn save(&self, data: bytes::Bytes) -> Result<PasteId, SaveError>;
    async fn load(
        &self,
        id: &PasteId,
    ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin>, LoadError>;
}

pub struct FilesystemStorage {
    root: PathBuf,
    id_gen: Box<dyn IdGen + Sync + Send>,
}

impl FilesystemStorage {
    pub fn new(root: impl Into<PathBuf>, id_gen: impl IdGen + Send + Sync + 'static) -> Self {
        Self {
            root: root.into(),
            id_gen: Box::new(id_gen),
        }
    }

    pub fn into_extension(self) -> StorageExtension {
        Extension(Arc::new(self))
    }
}

#[async_trait::async_trait]
impl Storage for FilesystemStorage {
    #[tracing::instrument(err, skip(self, data))]
    async fn save(&self, data: bytes::Bytes) -> Result<PasteId, SaveError> {
        let mut tmp = None;
        for attempt in 0..10 {
            let id = self.id_gen.next_id(attempt);

            let path = self.root.join(&id);
            if path.is_file() {
                continue;
            }
            tmp = Some((id, path));
            break;
        }

        let (id, path) = tmp.ok_or(SaveError)?;

        tracing::debug!("saving {id} at {}", path.display());

        let mut file = File::create(&path).await.map_err(|_| SaveError)?;
        file.write_all(&data).await.map_err(|_| SaveError)?;

        tracing::info!("saved {id} at {}", path.display());

        Ok(PasteId(id))
    }

    #[tracing::instrument(err, skip(self))]
    async fn load(
        &self,
        id: &PasteId,
    ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin>, LoadError> {
        let path = self.root.join(id.as_str());

        tracing::debug!("trying to load paste {id} from {}", path.display());

        let file = File::open(&path).await.map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => LoadError::NotFound,
            _ => LoadError::IoError(e),
        })?;

        tracing::info!("loaded paste {id} from {}", path.display());

        Ok(Box::new(file))
    }
}
