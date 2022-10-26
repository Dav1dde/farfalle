use std::{collections::BTreeMap, sync::Arc};

use axum::Extension;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, Highlighter, HtmlRenderer};

#[derive(Clone, Copy, Debug)]
pub enum Language {
    Bash,
    C,
    Cpp,
    Css,
    D,
    Go,
    Haskell,
    Html,
    Java,
    Javascript,
    Json,
    Lua,
    Python,
    Rust,
    Toml,
    Typescript,
    Tsx,
    Yaml,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "sh" => Some(Self::Bash),
            "zsh" => Some(Self::Bash),
            "c" => Some(Self::C),
            "h" => Some(Self::C),
            "cpp" => Some(Self::Cpp),
            "hpp" => Some(Self::Cpp),
            "css" => Some(Self::Css),
            "d" => Some(Self::D),
            "go" => Some(Self::Go),
            "hs" => Some(Self::Haskell),
            "lhs" => Some(Self::Haskell),
            "html" => Some(Self::Html),
            "xhtml" => Some(Self::Html),
            "java" => Some(Self::Java),
            "js" => Some(Self::Javascript),
            "jsx" => Some(Self::Javascript),
            "mjs" => Some(Self::Javascript),
            "json" => Some(Self::Json),
            "lua" => Some(Self::Lua),
            "py" => Some(Self::Python),
            "pyw" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "ts" => Some(Self::Typescript),
            "tsx" => Some(Self::Tsx),
            "yaml" => Some(Self::Yaml),
            _ => None,
        }
    }

    fn config(&self) -> HighlightConfiguration {
        match self {
            Self::Bash => pepegsitter::bash::highlight(),
            Self::C => pepegsitter::c::highlight(),
            Self::Cpp => pepegsitter::cpp::highlight(),
            Self::Css => pepegsitter::css::highlight(),
            Self::D => pepegsitter::d::highlight(),
            Self::Go => pepegsitter::go::highlight(),
            Self::Haskell => pepegsitter::haskell::highlight(),
            Self::Html => pepegsitter::html::highlight(),
            Self::Java => pepegsitter::java::highlight(),
            Self::Javascript => pepegsitter::javascript::highlight(),
            Self::Json => pepegsitter::json::highlight(),
            Self::Lua => pepegsitter::lua::highlight(),
            Self::Python => pepegsitter::python::highlight(),
            Self::Rust => pepegsitter::rust::highlight(),
            Self::Toml => pepegsitter::toml::highlight(),
            Self::Typescript => pepegsitter::typescript::highlight(),
            Self::Tsx => pepegsitter::tsx::highlight(),
            Self::Yaml => pepegsitter::yaml::highlight(),
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
