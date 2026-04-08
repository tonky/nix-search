#[cfg(not(target_arch = "wasm32"))]
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod output;
pub mod platform;
#[cfg(not(target_arch = "wasm32"))]
pub mod prep;
#[cfg(not(target_arch = "wasm32"))]
pub mod search;
#[cfg(not(target_arch = "wasm32"))]
pub mod tui;
pub mod types;

pub use nix_search_core::split;
