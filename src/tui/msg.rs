use crate::search::SearchResults;
use crate::types::{EnrichedDetails, EsConfig};

#[derive(Debug)]
pub enum Msg {
    Quit,
    Select,

    QueryAppend(char),
    QueryBackspace,
    QueryClear,

    MoveUp,
    MoveDown,
    TogglePane,
    TogglePlatform,

    ScrollDetailUp,
    ScrollDetailDown,

    ViewportRowsChanged(usize),

    ToggleHelp,
    OpenHomepage,

    SearchCompleted(SearchResults),
    SearchFailed(String),

    EsConfigResolved(Option<EsConfig>),
    EnrichmentLoaded(Option<EnrichedDetails>),
    EnrichmentFailed(String),
    CacheRefreshFinished(bool),
}
