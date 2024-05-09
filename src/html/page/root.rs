use crate::model::app_state::AppState;
use axum::extract::State;
use axum::response::Html;
use html_escaper::Escape;
use std::ops::Deref;
use std::sync::Arc;

#[axum::debug_handler]
pub async fn root(State(state): State<Arc<AppState>>) -> Html<String> {
    #[derive(boilerplate::Boilerplate)]
    #[boilerplate(filename = "web/html/page/root.html")]
    pub struct RootHtmlPage(Arc<AppState>);
    impl Deref for RootHtmlPage {
        type Target = AppState;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    let root_page = RootHtmlPage(state);
    Html(root_page.to_string())
}
