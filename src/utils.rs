use axum::extract::{FromRequest, RequestParts};
use hyper::{header, HeaderMap};
use serde::{de::value::StrDeserializer, Deserialize};

pub struct WithExtension<T>(pub T, pub Option<String>);

impl<'de, T> serde::de::Deserialize<'de> for WithExtension<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut s = String::deserialize(deserializer)?;

        let ext = match s.rfind('.') {
            Some(index) => {
                let ext = &s[index + 1..];
                let ext = (!ext.is_empty()).then(|| ext.to_owned());
                s.truncate(index);
                ext
            }
            None => None,
        };
        let value = T::deserialize(StrDeserializer::new(&s))?;

        Ok(Self(value, ext))
    }
}

pub struct Protocol(pub String);

#[async_trait::async_trait]
impl<B> FromRequest<B> for Protocol
where
    B: Send,
{
    type Rejection = ();

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        if let Some(protocol) = parse_forwarded_for_protocol(req.headers()) {
            return Ok(Self(protocol.to_owned()));
        }

        if let Some(protocol) = req
            .headers()
            .get("X-Forwarded-Proto")
            .and_then(|proto| proto.to_str().ok())
        {
            return Ok(Self(protocol.to_owned()));
        }

        if let Some(protocol) = req.uri().scheme_str() {
            return Ok(Self(protocol.to_owned()));
        }

        Ok(Self("http".to_owned()))
    }
}

fn parse_forwarded_for_protocol(headers: &HeaderMap) -> Option<&str> {
    let forwarded_values = headers.get(header::FORWARDED)?.to_str().ok()?;
    let first_value = forwarded_values.split(',').next()?;

    first_value.split(';').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("proto")
            .then(|| value.trim().trim_matches('"'))
    })
}
