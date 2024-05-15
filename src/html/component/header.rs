use crate::model::app_state::AppState;
use html_escaper::Escape;
use std::sync::Arc;

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "web/html/components/header.html")]
pub struct HeaderHtml {
    pub state: Arc<AppState>,
}
