use crate::{
    highlight::Highlighted,
    storage, templates,
    utils::{File, Protocol},
    Error, Language, PasteId, Result, StorageExtension, Theme, ThemeExtension, WithExtension,
    MAX_FILE_SIZE,
};
use axum::{
    extract::{ContentLengthLimit, Host, Multipart, Path},
    response::{Html, IntoResponse, Response},
    Extension,
};
use hyper::{header, HeaderMap};
use tokio::io::AsyncReadExt;

pub async fn root() -> impl IntoResponse {
    Html(templates::Index.to_string())
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

    let response = match File::infer(&data).map_err(|_| Error::StorageError)? {
        File::Binary(_, ft) => view_bin(ft, data)?.into_response(),
        File::Text(_, _) => {
            // safe because File::Text validates for UTF-8
            view_paste(&theme, unsafe { String::from_utf8_unchecked(data) }, ext)?.into_response()
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
                .filter(|ext| !ext.is_empty())
                .map(|x| x.to_owned());

            let data = field.bytes().await.map_err(|_| Error::BadRequest)?;
            if data.is_empty() {
                continue;
            }

            let guess_ext = File::infer(&data)?.extension();

            let id = storage.save(data).await.map_err(|_| Error::StorageError)?;
            let path = file_ext
                .as_deref()
                .or(guess_ext)
                .map(|ext| format!("{id}.{ext}"))
                .unwrap_or_else(|| id.into());

            let response = Response::builder()
                .status(303)
                .header("Location", &path)
                .body(format!("{protocol}://{host}/{path}\n"))
                .unwrap();
            return Ok(response);
        }
    }

    Err(Error::MissingFile)
}
