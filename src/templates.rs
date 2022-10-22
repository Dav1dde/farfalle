use askama::Template;

#[derive(Template, Default)]
#[template(path = "index.html")]
pub struct Index;

#[derive(Template, Default)]
#[template(path = "view.html")]
pub struct View<'a> {
    pub css: &'a str,
    pub source: &'a [&'a str],
    pub is_escaped: bool,
}
