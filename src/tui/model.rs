use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::search::{ScoredPackage, SearchResults};
use crate::types::{EnrichedDetails, EsConfig};

use super::cmd::WorkerTask;
use super::msg::Msg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    List,
    Detail,
}

pub struct Model {
    pub query: String,
    pub results: SearchResults,
    pub selected: usize,
    pub list_viewport_rows: usize,
    pub detail_scroll: usize,
    pub focus: Pane,
    pub platform: Option<String>,
    pub channel: String,

    pub should_quit: bool,
    pub selection: Option<String>,
    pub show_help: bool,
    pub tick: u64,

    pub cache_dir: PathBuf,
    pub es_config: Option<EsConfig>,
    pub enriched: Option<EnrichedDetails>,
    pub enriched_loading: bool,
    pub enriched_for: Option<String>,
    pub detail_error: Option<String>,

    pub cache_refreshing: bool,

    pub internal_tx: Sender<Msg>,
    pub internal_rx: Receiver<Msg>,
    pub worker_tx: Sender<WorkerTask>,
}

impl Model {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        initial_query: &str,
        platform: Option<String>,
        channel: &str,
        cache_dir: PathBuf,
        internal_tx: Sender<Msg>,
        internal_rx: Receiver<Msg>,
        worker_tx: Sender<WorkerTask>,
    ) -> Self {
        Self {
            query: initial_query.to_string(),
            results: SearchResults::default(),
            selected: 0,
            list_viewport_rows: 20,
            detail_scroll: 0,
            focus: Pane::List,
            platform,
            channel: channel.to_string(),
            should_quit: false,
            selection: None,
            show_help: false,
            tick: 0,
            cache_dir,
            es_config: None,
            enriched: None,
            enriched_loading: false,
            enriched_for: None,
            detail_error: None,
            cache_refreshing: false,
            internal_tx,
            internal_rx,
            worker_tx,
        }
    }

    pub fn flat_len(&self) -> usize {
        self.results.matched.len() + self.results.others.len()
    }

    pub fn selected_attr(&self) -> Option<String> {
        self.result_at(self.selected)
            .map(|sp| sp.package.attr_path.clone())
    }

    pub fn result_at(&self, index: usize) -> Option<&ScoredPackage> {
        if index < self.results.matched.len() {
            self.results.matched.get(index)
        } else {
            self.results
                .others
                .get(index.saturating_sub(self.results.matched.len()))
        }
    }

    pub fn clamp_selection(&mut self) {
        if self.flat_len() == 0 {
            self.selected = 0;
            return;
        }
        if self.selected >= self.flat_len() {
            self.selected = self.flat_len() - 1;
        }
    }

    pub fn set_list_viewport_rows(&mut self, rows: usize) -> bool {
        let rows = rows.max(1);
        if self.list_viewport_rows == rows {
            return false;
        }
        self.list_viewport_rows = rows;
        true
    }

    pub fn search_limit(&self) -> usize {
        // Reserve one row for the separator when platform grouping is active.
        if self.platform.is_some() {
            self.list_viewport_rows.saturating_sub(1).max(1)
        } else {
            self.list_viewport_rows.max(1)
        }
    }

    pub fn platform_separator_at(&self) -> Option<usize> {
        if self.results.others.is_empty() {
            None
        } else {
            Some(self.results.matched.len())
        }
    }
}
