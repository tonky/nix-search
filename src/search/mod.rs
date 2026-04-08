use crate::cache::index::NixIndex;
pub use nix_search_core::search::{ScoredPackage, SearchConfig, SearchResults};

pub mod filter;
pub mod query;

pub fn search(nix_index: &NixIndex, config: &SearchConfig) -> anyhow::Result<SearchResults> {
    let overfetch = nix_search_core::search::compute_overfetch_limit(config);

    let scored = query::run(nix_index, config, overfetch)?;
    let (mut matched, mut others) = filter::split_by_platform(scored, config.platform.as_deref());
    nix_search_core::search::apply_global_limit(&mut matched, &mut others, config.limit);

    Ok(SearchResults { matched, others })
}
