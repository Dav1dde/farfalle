use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use axum::Extension;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use tree_sitter_highlight::{Highlight, HighlightConfiguration, Highlighter, HtmlRenderer};

macro_rules! impl_language {
    ($(($lang:ident, $mod:ident, $($ext:expr),*),)+) => {
        #[derive(Clone, Copy, Debug)]
        pub enum Language {
            $($lang,)*
        }

        impl Language {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$lang => stringify!($mod),)*
                }
            }

            pub fn from_extension(ext: &str) -> Option<Self> {
                match ext {
                    $($($ext => Some(Self::$lang),)*)+
                    _ => None
                }
            }

            fn config(&self) -> HighlightConfiguration {
                match self {
                    $(Self::$lang => pepegsitter::$mod::highlight(),)*
                }
            }

            fn all() -> &'static [Self] {
                &[$(Self::$lang,)*]
            }
        }
    };
}

impl_language! {
    (Bash, bash, "sh", "zsh"),
    (C, c, "c", "h"),
    (Cpp, cpp, "cpp", "hpp"),
    (Css, css, "css"),
    (D, d, "d"),
    (Go, go, "go"),
    (Haskell, haskell, "hs", "lhs"),
    (Html, html, "html", "xhtml"),
    (Java, java, "java"),
    (JavaScript, javascript, "js", "jsx", "mjs"),
    (Json, json, "json"),
    (Lua, lua, "lua"),
    (Python, python, "py", "pyw"),
    (Rust, rust, "rs"),
    (Toml, toml, "toml"),
    (Typescript, typescript, "ts"),
    (Tsx, tsx, "tsx"),
    (Yaml, yaml, "yaml"),
}

pub struct Theme {
    name: String,
    styles: Styles,
    configs: HashMap<&'static str, HighlightConfiguration>,
}

impl Theme {
    fn new(name: String, styles: Styles) -> Self {
        let configs = Language::all()
            .iter()
            .map(|lang| {
                let mut config = lang.config();
                config.configure(&styles.highlight_names);

                (lang.as_str(), config)
            })
            .collect();

        Self {
            name,
            styles,
            configs,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn highlight(&self, language: Language, source: &str) -> Option<Highlighted> {
        let mut highlighter = Highlighter::new();

        let config = self.configs.get(language.as_str()).unwrap();

        let mut highlights = highlighter
            .highlight(config, source.as_bytes(), None, |lang| {
                self.configs.get(lang)
            })
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

impl<'de> serde::de::Deserialize<'de> for Theme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct T {
            name: String,
            theme: Styles,
        }

        let T { name, theme } = T::deserialize(deserializer)?;
        Ok(Self::new(name, theme))
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
