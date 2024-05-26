use crate::model::app_state::AppStateInner;
use html_escaper::Escape;

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "web/html/components/header.html")]
pub struct HeaderHtml<'a> {
    pub state: &'a AppStateInner,
}
