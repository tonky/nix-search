use crate::platform;

use super::cmd::Cmd;
use super::model::{Model, Pane};
use super::msg::Msg;

pub fn update(model: &mut Model, msg: Msg) -> anyhow::Result<Vec<Cmd>> {
    let cmds = match msg {
        Msg::Quit => {
            model.should_quit = true;
            vec![]
        }
        Msg::Select => {
            model.selection = model.selected_attr();
            model.should_quit = true;
            vec![]
        }
        Msg::QueryAppend(c) => {
            model.query.push(c);
            model.selected = 0;
            vec![run_search_cmd(model)]
        }
        Msg::QueryBackspace => {
            model.query.pop();
            model.selected = 0;
            vec![run_search_cmd(model)]
        }
        Msg::QueryClear => {
            model.query.clear();
            model.selected = 0;
            vec![run_search_cmd(model)]
        }
        Msg::MoveUp => {
            if model.selected > 0 {
                model.selected -= 1;
            }
            enrichment_cmd(model)
        }
        Msg::MoveDown => {
            if model.selected + 1 < model.flat_len() {
                model.selected += 1;
            }
            enrichment_cmd(model)
        }
        Msg::TogglePane => {
            model.focus = if model.focus == Pane::List {
                Pane::Detail
            } else {
                Pane::List
            };
            vec![]
        }
        Msg::TogglePlatform => {
            model.platform = if model.platform.is_some() {
                None
            } else {
                Some(platform::detect_current_platform())
            };
            model.selected = 0;
            vec![run_search_cmd(model)]
        }
        Msg::ScrollDetailUp => {
            model.detail_scroll = model.detail_scroll.saturating_sub(1);
            vec![]
        }
        Msg::ScrollDetailDown => {
            model.detail_scroll = model.detail_scroll.saturating_add(1);
            vec![]
        }
        Msg::ViewportRowsChanged(rows) => {
            if model.set_list_viewport_rows(rows) {
                model.selected = 0;
                vec![run_search_cmd(model)]
            } else {
                vec![]
            }
        }
        Msg::ToggleHelp => {
            model.show_help = !model.show_help;
            vec![]
        }
        Msg::OpenHomepage => vec![Cmd::OpenHomepage],
        Msg::SearchCompleted(results) => {
            model.results = results;
            model.selected = 0;
            model.clamp_selection();
            enrichment_cmd(model)
        }
        Msg::SearchFailed(err) => {
            model.detail_error = Some(err);
            vec![]
        }
        Msg::EsConfigResolved(cfg) => {
            model.es_config = cfg;
            enrichment_cmd(model)
        }
        Msg::EnrichmentLoaded(details) => {
            model.enriched_loading = false;
            model.detail_error = None;
            model.enriched = details.clone();
            model.enriched_for = details.map(|d| d.attr_path);
            vec![]
        }
        Msg::EnrichmentFailed(err) => {
            model.enriched_loading = false;
            model.detail_error = Some(err);
            vec![]
        }
        Msg::CacheRefreshFinished(_ok) => {
            model.cache_refreshing = false;
            vec![]
        }
    };

    Ok(cmds)
}

fn run_search_cmd(model: &Model) -> Cmd {
    Cmd::RunSearch {
        query: model.query.clone(),
        platform: model.platform.clone(),
        limit: model.search_limit(),
        exact_attr: None,
    }
}

fn enrichment_cmd(model: &mut Model) -> Vec<Cmd> {
    if let Some(attr) = model.selected_attr() {
        if model.enriched_for.as_deref() == Some(attr.as_str()) && model.enriched.is_some() {
            return vec![];
        }
        model.enriched_loading = true;
        model.detail_error = None;
        vec![Cmd::LoadEnrichment { attr_path: attr }]
    } else {
        model.enriched = None;
        model.enriched_loading = false;
        vec![]
    }
}
