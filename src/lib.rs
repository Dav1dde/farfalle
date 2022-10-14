mod error;
pub mod handler;
pub mod highlight;
pub mod id;
pub mod storage;
pub(crate) mod templates;
mod utils;

pub use self::error::{Error, Result};
pub use self::highlight::{Language, Theme};
pub use self::id::{IdGen, RandomIdGen};
pub use self::storage::{FilesystemStorage, PasteId, Storage};
pub use self::utils::WithExtension;

pub type StorageExtension = axum::Extension<std::sync::Arc<dyn Storage + Send + Sync>>;
pub type ThemeExtension = axum::Extension<std::sync::Arc<Theme>>;

pub const MAX_FILE_SIZE: u64 = 10 * 1000 * 1024;
