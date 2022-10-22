use std::{collections::BTreeMap, sync::Arc};

use axum::Extension;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, Highlighter, HtmlRenderer};

#[derive(Clone, Copy, Debug)]
pub enum Language {
    C,
    Cpp,
    Javascript,
    Json,
    Python,
    Rust,
    Typescript,
    TypescriptX,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "c" => Some(Self::C),
            "h" => Some(Self::C),
            "cpp" => Some(Self::Cpp),
            "hpp" => Some(Self::Cpp),
            "js" => Some(Self::Javascript),
            "mjs" => Some(Self::Javascript),
            "json" => Some(Self::Json),
            "py" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "ts" => Some(Self::Typescript),
            "tsx" => Some(Self::TypescriptX),
            _ => None,
        }
    }

    fn config(&self) -> HighlightConfiguration {
        match self {
            Self::C => HighlightConfiguration::new(
                tree_sitter_c::language(),
                tree_sitter_c::HIGHLIGHT_QUERY,
                "",
                "",
            )
            .unwrap(),
            Self::Cpp => HighlightConfiguration::new(
                tree_sitter_cpp::language(),
                tree_sitter_cpp::HIGHLIGHT_QUERY,
                "",
                "",
            )
            .unwrap(),
            Self::Javascript => HighlightConfiguration::new(
                tree_sitter_javascript::language(),
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_javascript::INJECTION_QUERY,
                tree_sitter_javascript::LOCALS_QUERY,
            )
            .unwrap(),
            Self::Json => HighlightConfiguration::new(
                tree_sitter_json::language(),
                tree_sitter_json::HIGHLIGHT_QUERY,
                "",
                "",
            )
            .unwrap(),
            Self::Python => HighlightConfiguration::new(
                tree_sitter_python::language(),
                tree_sitter_python::HIGHLIGHT_QUERY,
                "",
                "",
            )
            .unwrap(),
            Self::Rust => HighlightConfiguration::new(
                tree_sitter_rust::language(),
                tree_sitter_rust::HIGHLIGHT_QUERY,
                "",
                "",
            )
            .unwrap(),
            Self::Typescript => HighlightConfiguration::new(
                tree_sitter_typescript::language_typescript(),
                tree_sitter_typescript::HIGHLIGHT_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            )
            .unwrap(),
            Self::TypescriptX => HighlightConfiguration::new(
                tree_sitter_typescript::language_tsx(),
                tree_sitter_typescript::HIGHLIGHT_QUERY,
                "",
                tree_sitter_typescript::LOCALS_QUERY,
            )
            .unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Theme {
    name: String,
    #[serde(rename = "theme")]
    styles: Styles,
}

impl Theme {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn highlight(&self, language: Language, source: &str) -> Option<Highlighted> {
        let mut highlighter = Highlighter::new();

        let mut config = language.config();
        config.configure(&self.styles.highlight_names);

        let mut highlights = highlighter
            .highlight(&config, source.as_bytes(), None, |_| None)
            .ok()?;

        let mut renderer = HtmlRenderer::new();
        renderer
            .render(&mut highlights, source.as_bytes(), &|h| self.styles.attr(h))
            .ok()?;

        Some(Highlighted(renderer))
    }

    pub fn css(&self) -> &str {
        self.styles.css()
    }

    pub fn into_extension(self) -> Extension<Arc<Self>> {
        Extension(Arc::new(self))
    }
}

pub struct Highlighted(HtmlRenderer);

impl Highlighted {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.0.lines()
    }
}

#[derive(Debug)]
struct Styles {
    highlight_names: Vec<String>,
    styles: Vec<Style>,
    attrs: Vec<String>,
    css: OnceCell<String>,
}

impl Styles {
    fn new(theme: BTreeMap<String, Style>) -> Self {
        let mut highlight_names = Vec::with_capacity(theme.len());
        let mut styles = Vec::with_capacity(theme.len());
        let mut attrs = Vec::with_capacity(theme.len());

        for (i, (highlight_name, style)) in theme.into_iter().enumerate() {
            highlight_names.push(highlight_name);
            styles.push(style);
            attrs.push(format!("h{i}"));
        }

        Self {
            highlight_names,
            styles,
            attrs,
            css: OnceCell::new(),
        }
    }

    fn css(&self) -> &str {
        self.css.get_or_init(|| self.css_inner())
    }

    fn attr(&self, h: Highlight) -> &[u8] {
        self.attrs[h.0].as_bytes()
    }

    fn css_inner(&self) -> String {
        let mut css = String::new();

        for (i, style) in self.styles.iter().enumerate() {
            let attr = &self.attrs[i];
            css.push_str(&format!("[{attr}] {{"));
            css.push_str(&match style {
                Style::Color(color) => format!("color: {color};"),
                Style::Attributes(attrs) => attrs
                    .iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .join(";"),
            });
            css.push_str("}\n");
        }

        css
    }
}

impl<'de> serde::de::Deserialize<'de> for Styles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let theme = BTreeMap::<String, Style>::deserialize(deserializer)?;
        Ok(Self::new(theme))
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Style {
    Color(String),
    Attributes(BTreeMap<String, String>),
}
