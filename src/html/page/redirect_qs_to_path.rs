use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::RawQuery;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;

use super::not_found::not_found_html_response;
use crate::model::app_state::AppState;

/// This converts a query string into a path and redirects browser.
///
/// Purpose: enable a javascript-free website.
///
/// Problem being solved:
///
///     Our axum routes are all paths, however an HTML form submits user input as a query-string.
///     We need to convert that query string into a path.
///
///     We also want the end-user to see a nice path in the browser url
///     for purposes of copy/paste, etc.
///
///     Browser form submits:         We want:
///     /utxo?utxo=5&l=Submit         /utxo/5
///     /block?height=15&l=Submit     /block/height/15
///
/// Solution:
///
///     1. We submit all browser forms to /rqs with method=get.
///          (note: rqs is short for redirect-query-string)
///     2. /rqs calls this redirect_query_string_to_path() handler.
///     3. query-string is transformed to path as follows:
///        a) if _ig key is present, the value is split by ','
///           to obtain a list of query-string keys to ignore.
///        b. each key/val is converted to:
///              key     (if val is empty)
///              key/val (if val is not empty)
///        c) any keys in the _ig list are ignored.
///        d) keys and vals are url encoded
///        e) each resulting /key or /key/val is appended to the path.
///     4. a 301 redirect to the new path is sent to the browser.
///
/// An html form might look like:
///
///   <form action="/rqs" method="get">
///   <input type="hidden" name="block" value=""/>
///   <input type="hidden" name="_ig" value="l"/>
///
///   Block height or digest:
///   <input type="text" size="80" name="height_or_digest"/>
///   <input type="submit" name="l" value="Lookup Block"/>
///   </form>
///
/// note that the submit with name "l" is ignored because
/// of _ig=l.   We could ignore a list of fields also
/// eg _ig=field1,field2,field3,etc.
///
/// Order of keys in the query-string (and form) is important.
///
///   Any keys that are not ignored are translated into a
///   path in the order received.  Eg:
///     /rqs?block=&height_or_digest=10 --> /block/height_or_digest/10
///     /rqs?height_or_digest=10&block= --> /height_or_digest/10/block
///
/// A future enhancement could be to add an optional field for specifying the
/// path order.  That would enable re-ordering of inputs in a form without
/// altering the resulting path.  For now, our forms are so simple, that is not
/// needed.
#[axum::debug_handler]
pub async fn redirect_query_string_to_path(
    RawQuery(raw_query_option): RawQuery,
    State(state_rw): State<Arc<AppState>>,
) -> Result<Response, Response> {
    let state = &state_rw.load();

    let not_found = || not_found_html_response(state, None);

    let raw_query = raw_query_option.ok_or_else(not_found)?;

    // note: we construct a fake-url so we can use Url::query_pairs().
    let fake_url = format!("http://127.0.0.1/?{raw_query}");
    let url = url::Url::parse(&fake_url).map_err(|_| not_found())?;
    let query_vars: Vec<(String, _)> = url.query_pairs().into_owned().collect();

    const IGNORE_QS_VAR: &str = "_ig";

    let ignore_keys: HashSet<_> = match query_vars.iter().find(|(k, _)| k == IGNORE_QS_VAR) {
        Some((_k, v)) => v.split(',').collect(),
        None => Default::default(),
    };

    let mut new_path: String = Default::default();

    for (key, val) in query_vars.iter() {
        if key == IGNORE_QS_VAR || ignore_keys.contains(key as &str) {
            continue;
        }
        let parts = match val.is_empty() {
            false => format!("/{}/{}", url_encode(key), url_encode(val)),
            true => format!("/{}", url_encode(key)),
        };
        new_path += &parts;
    }

    match new_path.is_empty() {
        true => Err(not_found()),
        false => Ok(Redirect::permanent(&new_path).into_response()),
    }
}

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
