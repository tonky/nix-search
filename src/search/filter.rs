use super::ScoredPackage;

pub fn split_by_platform(
    packages: Vec<ScoredPackage>,
    platform: Option<&str>,
) -> (Vec<ScoredPackage>, Vec<ScoredPackage>) {
    nix_search_core::split::split_by_platform(packages, platform, |sp| &sp.package.platforms)
}
