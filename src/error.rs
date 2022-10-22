use axum::response::IntoResponse;
use hyper::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not found")]
    NotFound,

    #[error("bad request")]
    BadRequest,

    #[error("storage error")]
    StorageError,

    #[error("file contents are not recognized and not valid UTF-8")]
    NotUtf8,

    #[error("empty content")]
    Empty,

    #[error("unsupported file type '{0}'")]
    UnsupportedFile(&'static str),

    #[error("missing file")]
    MissingFile,
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::StorageError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotUtf8 => StatusCode::BAD_REQUEST,
            Self::Empty => StatusCode::BAD_REQUEST,
            Self::UnsupportedFile(..) => StatusCode::BAD_REQUEST,
            Self::MissingFile => StatusCode::BAD_REQUEST,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status_code(), self.to_string()).into_response()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
