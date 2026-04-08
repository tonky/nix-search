mod cache_sync;
mod search_runtime;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use nix_search_core::types::Package;
use wasm_bindgen_futures::spawn_local;

use crate::cache_sync::{RefreshStatus, StorageDiagnosticsReport, SyncStatus};
use crate::search_runtime::{SearchRow, run_search};

#[derive(Clone)]
struct MockRow {
    pkg: Package,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppState {
    StartupLoading,
    Ready,
    Error,
}

#[component]
fn App() -> impl IntoView {
    let rows = RwSignal::new(Vec::<SearchRow>::new());
    let query_raw = RwSignal::new(String::new());
    let query = RwSignal::new(String::new());
    let selected_attr = RwSignal::new(None::<String>);
    let cache_status = RwSignal::new("Cache status: initializing sync...".to_string());
    let app_state = RwSignal::new(AppState::StartupLoading);
    let refresh_loading = RwSignal::new(false);
    let refresh_progress_pct = RwSignal::new(0u8);
    let refresh_progress_detail = RwSignal::new("Starting refresh...".to_string());
    let diagnostics_loading = RwSignal::new(false);
    let diagnostics_visible = RwSignal::new(false);
    let diagnostics_report = RwSignal::new(None::<StorageDiagnosticsReport>);
    let reset_cache_loading = RwSignal::new(false);
    let reset_cache_result = RwSignal::new(None::<String>);
    let all_platforms = RwSignal::new(false);
    let selected_platform = RwSignal::new(None::<String>);
    let startup_latency_ms = RwSignal::new(None::<f64>);
    let last_search_latency_ms = RwSignal::new(None::<f64>);
    let startup_progress = RwSignal::new(0.02f64);
    let startup_message = RwSignal::new("Loading cache metadata...".to_string());
    let search_results = RwSignal::new(nix_search_core::search::SearchResults::default());
    let deferred_hydration_started = RwSignal::new(false);
    let deferred_hydration_running = RwSignal::new(false);

    let debounce_handle = Rc::new(RefCell::new(None::<Timeout>));

    Effect::new({
        let debounce_handle = Rc::clone(&debounce_handle);
        move |_| {
            let next_q = query_raw.get();
            if let Some(prev) = debounce_handle.borrow_mut().take() {
                prev.cancel();
            }
            let query_signal = query;
            let mut slot = debounce_handle.borrow_mut();
            *slot = Some(Timeout::new(120, move || {
                query_signal.set(next_q.clone());
            }));
        }
    });

    spawn_local(async move {
        let started = js_sys::Date::now();
        startup_message.set("Syncing local and remote cache...".to_string());
        startup_progress.set(0.10);
        match cache_sync::startup_status().await {
            Ok(status) => {
                cache_status.set(format_status(status));
                startup_message.set("Startup complete. Warming local index...".to_string());
                startup_progress.set(1.0);
                app_state.set(AppState::Ready);
            }
            Err(err) => {
                cache_status.set(format!(
                    "Cache status: startup degraded ({err}); continuing without local cache"
                ));
                startup_message.set("Startup degraded; local cache unavailable.".to_string());
                startup_progress.set(1.0);
                app_state.set(AppState::Ready);
            }
        }
        startup_latency_ms.set(Some(js_sys::Date::now() - started));
    });

    Effect::new(move |_| {
        if app_state.get() != AppState::Ready || deferred_hydration_started.get_untracked() {
            return;
        }

        deferred_hydration_started.set(true);
        deferred_hydration_running.set(true);
        startup_message.set("Loading local packages from browser cache...".to_string());
        startup_progress.set(0.12);

        let rows_sig = rows;
        let selected_attr_sig = selected_attr;
        let selected_platform_sig = selected_platform;
        let cache_status_sig = cache_status;
        let deferred_running_sig = deferred_hydration_running;

        spawn_local(async move {
            // Let first paint complete before background hydration starts.
            TimeoutFuture::new(180).await;

            match cache_sync::load_cached_packages_only().await {
                Ok(packages) => {
                    if packages.is_empty() {
                        cache_status_sig.set(
                            "Cache status: no local package cache yet; use Refresh Cache".to_string(),
                        );
                    } else {
                        apply_packages_async(
                            rows_sig,
                            selected_attr_sig,
                            selected_platform_sig,
                            Some((startup_message, startup_progress)),
                            packages,
                        )
                        .await;
                    }
                }
                Err(err) => {
                    let err_text = err.to_string();
                    let lower = err_text.to_lowercase();
                    if lower.contains("idb error")
                        || lower.contains("quota")
                        || lower.contains("notallowed")
                        || lower.contains("security")
                        || lower.contains("transactioninactive")
                    {
                        cache_status_sig.set(
                            format!(
                                "Cache status: browser storage unavailable for {}; refresh will run session-only",
                                browser_origin_label()
                            ),
                        );
                    } else {
                        cache_status_sig.set(format!(
                            "Cache status: failed to load local packages ({err})"
                        ));
                    }
                }
            }
            deferred_running_sig.set(false);
        });
    });

    let platforms = Memo::new(move |_| {
        rows.with(|all| {
            let mut out = BTreeSet::new();
            for r in all {
                for p in &r.pkg.platforms {
                    out.insert(p.clone());
                }
            }
            out.into_iter().collect::<Vec<_>>()
        })
    });

    Effect::new(move |_| {
        let started = js_sys::Date::now();
        let computed = rows.with(|all| {
            run_search(
                all,
                &query.get(),
                selected_platform.get().as_deref(),
                all_platforms.get(),
                120,
            )
        });
        search_results.set(computed);
        last_search_latency_ms.set(Some(js_sys::Date::now() - started));
    });

    let selected = move || {
        let target = selected_attr.get();
        search_results
            .get()
            .matched
            .into_iter()
            .chain(search_results.get().others)
            .into_iter()
            .map(|sp| MockRow { pkg: sp.package })
            .find(|r| Some(r.pkg.attr_path.clone()) == target)
    };

    Effect::new(move |_| {
        let current = selected_attr.get();
        let all = search_results
            .get()
            .matched
            .into_iter()
            .chain(search_results.get().others)
            .map(|sp| sp.package.attr_path)
            .collect::<Vec<_>>();
        let still_exists = current
            .as_ref()
            .map(|c| all.iter().any(|a| a == c))
            .unwrap_or(false);
        if !still_exists {
            let next = all.first().cloned();
            if current != next {
                selected_attr.set(next);
            }
        }
    });

    view! {
        <div id="app-root" class="app">
            <header class="header panel">
                <div>
                    <h1 class="title">"nix-search web shell"</h1>
                    <div class="status">{move || cache_status.get()}</div>
                </div>
                <div class="header-actions">
                    <button
                        class="diagnostics-btn"
                        prop:disabled=move || diagnostics_loading.get()
                        on:click={
                            move |_| {
                                if diagnostics_loading.get_untracked() {
                                    return;
                                }

                                diagnostics_visible.set(true);
                                diagnostics_loading.set(true);
                                reset_cache_result.set(None);

                                let diagnostics_loading_sig = diagnostics_loading;
                                let diagnostics_report_sig = diagnostics_report;

                                spawn_local(async move {
                                    let report = cache_sync::run_storage_diagnostics().await;
                                    diagnostics_report_sig.set(Some(report));
                                    diagnostics_loading_sig.set(false);
                                });
                            }
                        }
                    >
                        {move || {
                            if diagnostics_loading.get() {
                                "Running Diagnostics..."
                            } else {
                                "Storage Diagnostics"
                            }
                        }}
                    </button>

                    <button
                        class="refresh-btn"
                        prop:disabled=move || refresh_loading.get()
                        on:click={
                            move |_| {
                                if refresh_loading.get_untracked() {
                                    return;
                                }
                                refresh_loading.set(true);
                                refresh_progress_pct.set(3);
                                refresh_progress_detail.set("Checking manifest...".to_string());
                                cache_status.set("Cache status: refreshing cache...".to_string());

                                let rows_sig = rows;
                                let selected_attr_sig = selected_attr;
                                let selected_platform_sig = selected_platform;
                                let cache_status_sig = cache_status;
                                let refresh_loading_sig = refresh_loading;
                                let refresh_progress_pct_sig = refresh_progress_pct;
                                let refresh_progress_detail_sig = refresh_progress_detail;

                                spawn_local(async move {
                                    match cache_sync::force_refresh_with_progress(move |progress| {
                                        refresh_progress_pct_sig.set(progress.percent);
                                        refresh_progress_detail_sig.set(progress.detail);
                                    })
                                    .await
                                    {
                                        Ok((packages, outcome)) => {
                                            if !packages.is_empty() {
                                                apply_packages_async(
                                                    rows_sig,
                                                    selected_attr_sig,
                                                    selected_platform_sig,
                                                    None,
                                                    packages,
                                                )
                                                .await;
                                            }
                                            cache_status_sig.set(format_refresh_status(outcome));
                                            app_state.set(AppState::Ready);
                                        }
                                        Err(err) => {
                                            refresh_progress_pct_sig.set(100);
                                            refresh_progress_detail_sig
                                                .set("Refresh failed".to_string());
                                            cache_status_sig.set(format!(
                                                "Cache status: refresh failed ({err})"
                                            ));
                                            app_state.set(AppState::Error);
                                        }
                                    }
                                    refresh_loading_sig.set(false);
                                });
                            }
                        }
                    >
                        {move || {
                            if refresh_loading.get() {
                                "Refreshing..."
                            } else {
                                "Refresh Cache"
                            }
                        }}
                    </button>
                </div>
            </header>

            {move || {
                if refresh_loading.get() {
                    let pct = refresh_progress_pct.get();
                    view! {
                        <section class="refresh-progress panel" aria-label="Cache refresh progress">
                            <p class="refresh-progress-label">{move || format!("{} ({}%)", refresh_progress_detail.get(), refresh_progress_pct.get())}</p>
                            <div
                                class="refresh-progress-track"
                                role="progressbar"
                                aria-valuemin="0"
                                aria-valuemax="100"
                                aria-valuenow={pct as i32}
                                aria-valuetext=move || format!("{} percent", refresh_progress_pct.get())
                            >
                                <div
                                    class="refresh-progress-fill"
                                    style=move || format!("width: {}%;", refresh_progress_pct.get())
                                ></div>
                            </div>
                        </section>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || {
                if diagnostics_visible.get() {
                    view! {
                        <section class="diagnostics-panel panel" aria-label="Storage diagnostics panel">
                            <div class="diagnostics-header">
                                <h2>"Storage diagnostics"</h2>
                                <div class="diagnostics-actions">
                                    <button
                                        class="diagnostics-reset"
                                        prop:disabled=move || reset_cache_loading.get()
                                        on:click={
                                            move |_| {
                                                if reset_cache_loading.get_untracked() {
                                                    return;
                                                }

                                                reset_cache_loading.set(true);
                                                reset_cache_result.set(None);
                                                rows.set(Vec::new());
                                                selected_attr.set(None);
                                                selected_platform.set(None);
                                                cache_status.set(format!(
                                                    "Cache status: resetting local browser cache for {}...",
                                                    browser_origin_label()
                                                ));

                                                let reset_cache_loading_sig = reset_cache_loading;
                                                let reset_cache_result_sig = reset_cache_result;
                                                let diagnostics_report_sig = diagnostics_report;
                                                let cache_status_sig = cache_status;

                                                spawn_local(async move {
                                                    // Yield once so UI updates render before heavy IDB work.
                                                    TimeoutFuture::new(0).await;

                                                    match cache_sync::reset_local_cache().await {
                                                        Ok(()) => {
                                                            diagnostics_report_sig.set(None);
                                                            reset_cache_result_sig.set(Some(
                                                                "Local browser cache reset. Run Storage Diagnostics and then Refresh Cache."
                                                                    .to_string(),
                                                            ));
                                                            cache_status_sig.set(format!(
                                                                "Cache status: local browser cache reset for {}; click Refresh Cache",
                                                                browser_origin_label()
                                                            ));
                                                        }
                                                        Err(err) => {
                                                            reset_cache_result_sig.set(Some(format!(
                                                                "Cache reset failed: {err}"
                                                            )));
                                                        }
                                                    }
                                                    reset_cache_loading_sig.set(false);
                                                });
                                            }
                                        }
                                    >
                                        {move || {
                                            if reset_cache_loading.get() {
                                                "Resetting..."
                                            } else {
                                                "Reset Local Cache"
                                            }
                                        }}
                                    </button>

                                    <button
                                        class="diagnostics-close"
                                        on:click=move |_| diagnostics_visible.set(false)
                                    >
                                        "Close"
                                    </button>
                                </div>
                            </div>
                            {move || {
                                if diagnostics_loading.get() {
                                    return view! {
                                        <p class="diagnostics-loading">"Running storage probes..."</p>
                                    }
                                    .into_any();
                                }

                                match diagnostics_report.get() {
                                    Some(report) => {
                                        let notes = report.notes;
                                        view! {
                                            <dl class="diagnostics-grid">
                                                <dt>"Current origin"</dt>
                                                <dd>{report.current_origin.clone()}</dd>

                                                <dt>"Secure context"</dt>
                                                <dd>{bool_label(report.secure_context)}</dd>

                                                <dt>"StorageManager API"</dt>
                                                <dd>{bool_label(Some(report.storage_manager_available))}</dd>

                                                <dt>"Persistent already enabled"</dt>
                                                <dd>{bool_label(report.persisted)}</dd>

                                                <dt>"persist() request granted"</dt>
                                                <dd>{bool_label(report.persist_granted)}</dd>

                                                <dt>"Storage usage"</dt>
                                                <dd>{format_storage_bytes(report.estimate_usage_bytes)}</dd>

                                                <dt>"Storage quota"</dt>
                                                <dd>{format_storage_bytes(report.estimate_quota_bytes)}</dd>

                                                <dt>"IndexedDB write probe"</dt>
                                                <dd>{bool_label(Some(report.indexeddb_write_ok))}</dd>
                                            </dl>

                                            <p class="diagnostics-error">
                                                {report
                                                    .indexeddb_error
                                                    .map(|e| format!("IDB error: {e}"))
                                                    .unwrap_or_else(|| "IDB error: none".to_string())}
                                            </p>

                                            <ul class="diagnostics-notes">
                                                {notes
                                                    .into_iter()
                                                    .map(|note| view! { <li>{note}</li> })
                                                    .collect_view()}
                                            </ul>
                                        }
                                        .into_any()
                                    }
                                    None => view! {
                                        <p class="diagnostics-loading">
                                            "Run diagnostics to inspect browser storage behavior."
                                        </p>
                                    }
                                    .into_any(),
                                }
                            }}
                            {move || {
                                reset_cache_result
                                    .get()
                                    .map(|msg| {
                                        view! { <p class="diagnostics-reset-status">{msg}</p> }
                                            .into_any()
                                    })
                                    .unwrap_or_else(|| view! { <></> }.into_any())
                            }}
                        </section>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || {
                if app_state.get() == AppState::StartupLoading {
                    view! {
                        <section class="startup-overlay panel" aria-label="Startup progress">
                            <h2 class="startup-title">"Preparing local search"</h2>
                            <p class="startup-message">{move || startup_message.get()}</p>
                            <div class="startup-progress-track" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow=move || (startup_progress.get() * 100.0).round() as i32>
                                <div
                                    class="startup-progress-fill"
                                    style=move || {
                                        let width = (startup_progress.get() * 100.0).clamp(0.0, 100.0);
                                        format!("width: {width:.1}%;")
                                    }
                                ></div>
                            </div>
                            <p class="startup-progress-label">
                                {move || {
                                    let pct = (startup_progress.get() * 100.0).clamp(0.0, 100.0);
                                    format!("{pct:.0}%")
                                }}
                            </p>
                        </section>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <main class="layout">
                <section class="panel left-pane" aria-label="Search results pane">
                    <div class="search-wrap">
                        <input
                            class="search-input"
                            type="search"
                            placeholder="Search packages"
                            prop:value=move || query_raw.get()
                            on:input=move |ev| query_raw.set(event_target_value(&ev))
                        />
                        <div class="search-controls">
                            <label class="platform-toggle">
                                <input
                                    type="checkbox"
                                    prop:checked=move || all_platforms.get()
                                    on:change=move |ev| all_platforms.set(event_target_checked(&ev))
                                />
                                "All platforms"
                            </label>
                            <select
                                class="platform-select"
                                prop:disabled=move || all_platforms.get()
                                on:change=move |ev| selected_platform.set(Some(event_target_value(&ev)))
                            >
                                <For
                                    each=move || platforms.get()
                                    key=|p| p.clone()
                                    children=move |p: String| {
                                        let p_for_selected = p.clone();
                                        let p_for_value = p.clone();
                                        let selected =
                                            move || selected_platform.get() == Some(p_for_selected.clone());
                                        view! {
                                            <option value={p_for_value} selected=selected>{p}</option>
                                        }
                                    }
                                />
                            </select>
                        </div>
                        {move || {
                            if deferred_hydration_running.get() {
                                let pct = (startup_progress.get() * 100.0).clamp(0.0, 100.0);
                                view! {
                                    <div class="deferred-hydration-hint">
                                        <span class="deferred-hydration-dot" aria-hidden="true"></span>
                                        "Warming local index in background..."
                                    </div>
                                    <div class="deferred-progress-track" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct.round() as i32}>
                                        <div
                                            class="deferred-progress-fill"
                                            style={format!("width: {pct:.1}%;")}
                                        ></div>
                                    </div>
                                }
                                .into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                    </div>
                    <ul class="results">
                        {move || {
                            if app_state.get() == AppState::StartupLoading {
                                return view! {
                                    <li class="results-empty">{move || startup_message.get()}</li>
                                }
                                .into_any();
                            }

                            if deferred_hydration_running.get() && rows.get().is_empty() {
                                return view! {
                                    <li class="results-empty">
                                        "Loading local search index in background..."
                                    </li>
                                }
                                .into_any();
                            }

                            let r = search_results.get();
                            let matched_rows = r
                                .matched
                                .into_iter()
                                .map(|sp| MockRow { pkg: sp.package })
                                .collect::<Vec<_>>();
                            let others_rows = r
                                .others
                                .into_iter()
                                .map(|sp| MockRow { pkg: sp.package })
                                .collect::<Vec<_>>();

                            if matched_rows.is_empty() && others_rows.is_empty() {
                                return view! {
                                    <li class="results-empty">
                                        "No packages found for this query."
                                    </li>
                                }
                                .into_any();
                            }

                            view! {
                                <>
                                    <li class="results-separator">"Matched"</li>
                                    <For
                                        each=move || matched_rows.clone()
                                        key=|row| row.pkg.attr_path.clone()
                                        children=move |row| {
                                            render_row(row, selected_attr)
                                        }
                                    />
                                    {if others_rows.is_empty() {
                                        view! { <></> }.into_any()
                                    } else {
                                        view! {
                                            <>
                                                <li class="results-separator">"Other Platforms"</li>
                                                <For
                                                    each=move || others_rows.clone()
                                                    key=|row| row.pkg.attr_path.clone()
                                                    children=move |row| {
                                                        render_row(row, selected_attr)
                                                    }
                                                />
                                            </>
                                        }
                                        .into_any()
                                    }}
                                </>
                            }
                            .into_any()
                        }}
                    </ul>
                </section>

                <section class="panel right-pane" aria-label="Package detail pane">
                    {move || match app_state.get() {
                        AppState::StartupLoading => view! {
                            <div class="detail-empty">
                                <h2>"Initializing"</h2>
                                <p>{move || startup_message.get()}</p>
                            </div>
                        }
                        .into_any(),
                        AppState::Error if rows.get().is_empty() => view! {
                            <div class="detail-empty">
                                <h2>"Cache unavailable"</h2>
                                <p>"Startup sync failed and no local cache is available yet."</p>
                            </div>
                        }
                        .into_any(),
                        _ => match selected() {
                        Some(row) => view! {
                            <div>
                                <h2>{row.pkg.attr_path}</h2>
                                <p>{row.pkg.description}</p>
                                <p class="meta">{format!("Version: {}", row.pkg.version)}</p>
                                <h3>"Platforms"</h3>
                                <div>
                                    {row.pkg.platforms.into_iter().map(|p| view! { <span class="badge">{p}</span> }).collect_view()}
                                </div>
                            </div>
                        }
                            .into_any(),
                        None => view! {
                            <div class="detail-empty">
                                <h2>"No selection"</h2>
                                <p>"Pick a package from the list to see details."</p>
                            </div>
                        }
                            .into_any(),
                    }}}
                </section>
            </main>

            <footer class="perf-strip">
                <span>
                    {move || {
                        startup_latency_ms
                            .get()
                            .map(|ms| format!("startup: {:.1} ms", ms))
                            .unwrap_or_else(|| "startup: n/a".to_string())
                    }}
                </span>
                <span>
                    {move || {
                        last_search_latency_ms
                            .get()
                            .map(|ms| format!("search: {:.1} ms", ms))
                            .unwrap_or_else(|| "search: n/a".to_string())
                    }}
                </span>
                <span>
                    {move || format!("rows: {}", rows.get().len())}
                </span>
            </footer>
        </div>
    }
}

async fn apply_packages_async(
    rows: RwSignal<Vec<SearchRow>>,
    selected_attr: RwSignal<Option<String>>,
    selected_platform: RwSignal<Option<String>>,
    progress: Option<(RwSignal<String>, RwSignal<f64>)>,
    packages: Vec<Package>,
) {
    if packages.is_empty() {
        rows.set(Vec::new());
        selected_attr.set(None);
        selected_platform.set(None);
        return;
    }

    let total = packages.len();
    let mut mapped = Vec::with_capacity(total);
    const CHUNK_SIZE: usize = 200;
    const PROGRESS_UPDATE_EVERY: usize = 1000;

    set_progress(
        &progress,
        "Preparing local search index...".to_string(),
        0.22,
    );

    for (idx, pkg) in packages.into_iter().enumerate() {
        mapped.push(SearchRow::from_package(pkg));

        let done = idx + 1;
        if done % CHUNK_SIZE == 0 {
            if done % PROGRESS_UPDATE_EVERY == 0 {
                let frac = done as f64 / total as f64;
                let scaled = 0.22 + frac * 0.70;
                set_progress(
                    &progress,
                    format!("Preparing local search index... {done}/{total}"),
                    scaled,
                );
            }
            TimeoutFuture::new(0).await;
        }
    }

    set_progress(&progress, "Finalizing package index...".to_string(), 0.95);
    mapped.sort_by(|a, b| a.pkg.attr_path.cmp(&b.pkg.attr_path));

    let first_attr = mapped.first().map(|r| r.pkg.attr_path.clone());

    let first_platform = mapped
        .iter()
        .flat_map(|r| r.pkg.platforms.iter())
        .next()
        .cloned();

    let preferred_platform = detect_browser_platform();
    let effective_platform = preferred_platform
        .filter(|p| mapped.iter().any(|r| r.pkg.platforms.iter().any(|rp| rp == p)))
        .or(first_platform);

    rows.set(mapped);
    selected_attr.set(first_attr);
    selected_platform.set(effective_platform);
    set_progress(&progress, "Startup complete.".to_string(), 1.0);
}

fn set_progress(progress: &Option<(RwSignal<String>, RwSignal<f64>)>, message: String, pct: f64) {
    if let Some((signal_message, signal_progress)) = progress {
        signal_message.set(message);
        signal_progress.set(pct.clamp(0.0, 1.0));
    }
}

fn detect_browser_platform() -> Option<String> {
    let ua = web_sys::window()?
        .navigator()
        .user_agent()
        .ok()?
        .to_lowercase();

    if ua.contains("mac os x") || ua.contains("macintosh") {
        if ua.contains("arm") || ua.contains("aarch64") || ua.contains("apple silicon") {
            return Some("aarch64-darwin".to_string());
        }
        return Some("x86_64-darwin".to_string());
    }

    if ua.contains("linux") {
        if ua.contains("aarch64") || ua.contains("arm64") {
            return Some("aarch64-linux".to_string());
        }
        return Some("x86_64-linux".to_string());
    }

    None
}

fn render_row(row: MockRow, selected_attr: RwSignal<Option<String>>) -> impl IntoView {
    let attr = row.pkg.attr_path.clone();
    let is_active = Memo::new(move |_| selected_attr.get() == Some(attr.clone()));
    view! {
        <li>
            <button
                class=("result-item", true)
                class:active=move || is_active.get()
                on:click={
                    let attr_click = row.pkg.attr_path.clone();
                    move |_| selected_attr.set(Some(attr_click.clone()))
                }
            >
                <span class="attr">{row.pkg.attr_path.clone()}</span>
                <span class="meta">{format!("{}  |  {}", row.pkg.version, row.pkg.pname)}</span>
            </button>
        </li>
    }
}

fn format_status(status: SyncStatus) -> String {
    match status {
        SyncStatus::UpToDate(meta) => format!(
            "Cache status: up to date {} ({} packages)",
            meta.version, meta.package_count
        ),
        SyncStatus::UpdateAvailable(manifest) => format!(
            "Cache status: update available {} ({} packages), local load deferred",
            manifest.version, manifest.package_count
        ),
        SyncStatus::OfflineUsingLocal(meta) => format!(
            "Cache status: offline, using local {} ({} packages)",
            meta.version, meta.package_count
        ),
        SyncStatus::NoCacheOffline => {
            "Cache status: offline and no local cache available".to_string()
        }
    }
}

fn format_refresh_status(status: RefreshStatus) -> String {
    match status {
        RefreshStatus::Updated(meta) => format!(
            "Cache status: refresh updated to {} ({} packages)",
            meta.version, meta.package_count
        ),
        RefreshStatus::UpdatedInMemory {
            version,
            package_count,
            reason,
            } => {
                if reason.contains("IndexedDB writes unavailable") {
                    format!(
                        "Cache status: refreshed to {} ({} packages), session-only (browser storage unavailable)",
                        version, package_count
                    )
                } else {
                    format!(
                        "Cache status: refreshed to {} ({} packages), session-only ({})",
                        version, package_count, reason
                    )
                }
            }
        RefreshStatus::UpToDate(meta) => format!(
            "Cache status: already up to date {} ({} packages)",
            meta.version, meta.package_count
        ),
        RefreshStatus::Failed(message) => {
            format!("Cache status: refresh failed ({message}); using previous cache")
        }
    }
}

fn bool_label(value: Option<bool>) -> String {
    match value {
        Some(true) => "yes".to_string(),
        Some(false) => "no".to_string(),
        None => "n/a".to_string(),
    }
}

fn format_storage_bytes(value: Option<f64>) -> String {
    let Some(bytes) = value else {
        return "n/a".to_string();
    };

    let gib = 1024.0 * 1024.0 * 1024.0;
    let mib = 1024.0 * 1024.0;
    if bytes >= gib {
        format!("{:.2} GiB", bytes / gib)
    } else {
        format!("{:.1} MiB", bytes / mib)
    }
}

fn browser_origin_label() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "this origin".to_string())
}

pub fn mount_app() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
