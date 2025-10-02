use html_escaper::Escape;

use crate::model::app_state::AppStateInner;

#[derive(Debug, Clone, boilerplate::Boilerplate)]
#[boilerplate(filename = "web/html/components/header.html")]
pub struct HeaderHtml<'a> {
    pub state: &'a AppStateInner,
}
