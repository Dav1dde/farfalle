use crate::{
    highlight::Highlighted, storage, templates, utils::Protocol, Error, Language, PasteId, Result,
    StorageExtension, Theme, ThemeExtension, WithExtension, MAX_FILE_SIZE,
};
use axum::{
    extract::{ContentLengthLimit, Host, Multipart, Path},
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};
use hyper::{header, HeaderMap};
use tokio::io::AsyncReadExt;

pub async fn root() -> &'static str {
    "Farfalle"
}

pub async fn view(
    Path(WithExtension(id, ext)): Path<WithExtension<PasteId>>,
    Extension(storage): StorageExtension,
    Extension(theme): ThemeExtension,
) -> Result<impl IntoResponse> {
    let mut data = Vec::new();

    storage
        .load(&id)
        .await
        .map_err(|e| match e {
            storage::LoadError::NotFound => Error::NotFound,
            _ => Error::StorageError,
        })?
        .read_to_end(&mut data)
        .await
        .map_err(|_| Error::StorageError)?;

    let response = match infer::get(&data) {
        Some(ft) => view_bin(ft, data)?.into_response(),
        None => {
            let content = String::from_utf8(data).map_err(|_| Error::StorageError)?;
            view_paste(&theme, content, ext)?.into_response()
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=31536000".parse().unwrap(),
    );

    Ok((headers, response))
}

fn view_bin(ft: infer::Type, data: Vec<u8>) -> Result<impl IntoResponse> {
    let content_type = match ft.matcher_type() {
        // Surely this will never fail
        infer::MatcherType::Image => ft.mime_type().parse().unwrap(),
        // All other content types are not supported and an error,
        // this should never happen because it is checked on upload.
        _ => return Err(Error::StorageError),
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type);

    Ok((headers, data))
}

fn view_paste(theme: &Theme, source: String, ext: Option<String>) -> Result<impl IntoResponse> {
    let language = ext.map(|ext| Language::from_extension(&ext));

    let highlighted = |highlighted: Highlighted| {
        let lines = highlighted.lines().collect::<Vec<_>>();

        templates::View {
            css: theme.css(),
            source: &lines,
            is_escaped: true,
        }
        .to_string()
    };
    let html = || {
        templates::View {
            source: &source
                .lines()
                .map(|line| if line.is_empty() { "\n" } else { line })
                .collect::<Vec<_>>(),
            ..Default::default()
        }
        .to_string()
    };

    let response = match language {
        Some(Some(language)) => {
            let v = theme
                .highlight(language, &source)
                .map(highlighted)
                .unwrap_or_else(html);
            Html(v).into_response()
        }
        Some(None) => Html(html()).into_response(),
        None => source.into_response(),
    };

    Ok(response)
}

pub async fn upload(
    Protocol(protocol): Protocol,
    Host(host): Host,
    ContentLengthLimit(mut data): ContentLengthLimit<Multipart, MAX_FILE_SIZE>,
    Extension(storage): StorageExtension,
) -> Result<impl IntoResponse> {
    while let Some(field) = data.next_field().await.map_err(|_| Error::BadRequest)? {
        if field.name() == Some("file") {
            let file_ext = field
                .file_name()
                .and_then(|f| f.rsplit('.').next())
                .map(|x| x.to_owned());

            let (data, guess_ext) = validate(field.bytes().await.map_err(|_| Error::BadRequest)?)?;

            let id = storage.save(data).await.map_err(|_| Error::StorageError)?;

            let r = if let Some(ext) = file_ext.as_deref().or(guess_ext) {
                format!("{protocol}://{host}/{id}.{ext}\n")
            } else {
                format!("{protocol}://{host}/{id}\n")
            };
            return Ok((StatusCode::OK, r));
        }
    }

    Err(Error::MissingFile)
}

fn validate(bytes: bytes::Bytes) -> Result<(bytes::Bytes, Option<&'static str>)> {
    let ext = match infer::get(&bytes) {
        Some(ft) => match ft.matcher_type() {
            infer::MatcherType::Image => Some(ft.extension()),
            _ => return Err(Error::UnsupportedFile(ft.mime_type())),
        },
        None => {
            std::str::from_utf8(&bytes)
                .map(|_| ())
                .map_err(|_| Error::NotUtf8)?;
            None
        }
    };

    Ok((bytes, ext))
}
