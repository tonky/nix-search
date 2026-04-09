use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use js_sys::{Function, Promise, Reflect};
use nix_search_core::types::Package;
use rexie::{ObjectStore, Rexie, TransactionMode};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::collections::HashSet;
use thiserror::Error;
use js_sys::wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::console;

const DB_NAME: &str = "nix-search-web";
const DB_VERSION: u32 = 1;
const STORE_PACKAGES: &str = "packages";
const STORE_META: &str = "meta";
const META_KEY_CURRENT: &str = "current";
const META_KEY_DIAGNOSTICS_PROBE: &str = "__storage_diag_probe__";
const REMOTE_MANIFEST_CANDIDATES: &[&str] = &["data/manifest.json", "/data/manifest.json", "manifest.json"];
const ENABLE_STALE_PRUNE_ON_REFRESH: bool = false;

#[derive(Debug, Clone)]
pub struct StorageDiagnosticsReport {
    pub current_origin: String,
    pub secure_context: Option<bool>,
    pub storage_manager_available: bool,
    pub persisted: Option<bool>,
    pub persist_granted: Option<bool>,
    pub estimate_usage_bytes: Option<f64>,
    pub estimate_quota_bytes: Option<f64>,
    pub indexeddb_package_entries: Option<usize>,
    pub indexeddb_meta_entries: Option<usize>,
    pub local_storage_entries: Option<usize>,
    pub session_storage_entries: Option<usize>,
    pub cache_storage_entries: Option<usize>,
    pub indexeddb_write_ok: bool,
    pub indexeddb_error: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub key: String,
    pub version: String,
    pub checksum: String,
    pub package_count: usize,
    pub built_at: u64,
    pub artifact: String,
    pub fetched_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteManifest {
    pub version: String,
    pub checksum: String,
    pub package_count: usize,
    pub built_at: u64,
    pub artifact: String,
    pub compressed_artifact: Option<String>,
    pub compressed_format: Option<String>,
    pub compressed_size_bytes: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreparedData {
    packages: Vec<Package>,
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    UpToDate(CacheMeta),
    UpdateAvailable(RemoteManifest),
    OfflineUsingLocal(CacheMeta),
    NoCacheOffline,
}

#[derive(Debug, Clone)]
pub enum RefreshStatus {
    Updated(CacheMeta),
    UpdatedInMemory {
        version: String,
        package_count: usize,
        reason: String,
    },
    UpToDate(CacheMeta),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct RefreshProgress {
    pub percent: u8,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("failed to open IndexedDB: {0}")]
    OpenDb(String),
    #[error("failed database transaction: {0}")]
    DbTx(String),
    #[error("failed to serialize/deserialize data: {0}")]
    Serde(String),
    #[error("network request failed: {0}")]
    Network(String),
}

pub async fn startup_status() -> Result<SyncStatus, CacheError> {
    let local_meta = match open_db().await {
        Ok(db) => match load_meta(&db).await {
            Ok(meta) => meta,
            Err(err) => {
                log_debug(&format!("startup: load_meta failed, continuing without local meta: {err}"));
                None
            }
        },
        Err(err) => {
            log_debug(&format!("startup: open_db failed, continuing without local meta: {err}"));
            None
        }
    };

    match fetch_remote_manifest().await {
        Ok(manifest) => {
            let is_same_version = local_meta
                .as_ref()
                .map(|m| m.version == manifest.0.version)
                .unwrap_or(false);

            if is_same_version {
                let meta = local_meta.expect("meta exists when version matched");
                Ok(SyncStatus::UpToDate(meta))
            } else {
                Ok(SyncStatus::UpdateAvailable(manifest.0))
            }
        }
        Err(_) => {
            if let Some(meta) = local_meta {
                Ok(SyncStatus::OfflineUsingLocal(meta))
            } else {
                Ok(SyncStatus::NoCacheOffline)
            }
        }
    }
}

pub async fn load_cached_packages_only() -> Result<Vec<Package>, CacheError> {
    let db = open_db().await?;
    load_packages(&db).await
}

pub async fn force_refresh_with_progress<F>(mut on_progress: F) -> Result<(Vec<Package>, RefreshStatus), CacheError>
where
    F: FnMut(RefreshProgress),
{
    emit_refresh(&mut on_progress, 3, "Checking manifest...");

    let db = open_db().await.ok();

    // Avoid large/fragile pre-refresh reads on Firefox. Refresh primarily needs
    // remote manifest + artifact + write path; existing UI state is retained if
    // we return an empty package list.
    let local_meta = if let Some(db_ref) = db.as_ref() {
        load_meta(db_ref).await.ok().flatten()
    } else {
        None
    };

    log_debug("refresh: fetching manifest");
    let (manifest, manifest_source) = match fetch_remote_manifest().await {
        Ok(m) => m,
        Err(err) => {
            log_debug(&format!("refresh: manifest fetch failed: {err}"));
            emit_refresh(&mut on_progress, 100, "Manifest check failed");
            return Ok((
                Vec::new(),
                RefreshStatus::Failed(format!("manifest check failed: {err}")),
            ));
        }
    };

    let is_same_version = local_meta
        .as_ref()
        .map(|m| m.version == manifest.version)
        .unwrap_or(false);

    if is_same_version {
        log_debug("refresh: manifest version unchanged");
        let meta = local_meta.expect("meta exists when version matched");
        emit_refresh(&mut on_progress, 100, "Already up to date");
        return Ok((Vec::new(), RefreshStatus::UpToDate(meta)));
    }

    let artifact_candidates = artifact_candidates(&manifest, &manifest_source);

    emit_refresh(&mut on_progress, 18, "Downloading package snapshot...");
    let mut prepared = None;
    let mut last_err = None;

    for candidate in &artifact_candidates {
        log_debug(&format!(
            "refresh: downloading artifact from '{}' (compressed={})",
            candidate.url, candidate.compressed
        ));
        match fetch_artifact(candidate).await {
            Ok(p) => {
                prepared = Some(p);
                break;
            }
            Err(err) => {
                log_debug(&format!(
                    "refresh: artifact candidate failed '{}' (compressed={}): {err}",
                    candidate.url, candidate.compressed
                ));
                last_err = Some(err.to_string());
            }
        }
    }

    let prepared = match prepared {
        Some(p) => p,
        None => {
            emit_refresh(&mut on_progress, 100, "Snapshot download failed");
            return Ok((
                Vec::new(),
                RefreshStatus::Failed(format!(
                    "download failed: {}",
                    last_err.unwrap_or_else(|| "all artifact candidates failed".to_string())
                )),
            ));
        }
    };

    let new_meta = CacheMeta {
        key: META_KEY_CURRENT.to_string(),
        version: manifest.version,
        checksum: manifest.checksum,
        package_count: manifest.package_count,
        built_at: manifest.built_at,
        artifact: manifest.artifact,
        fetched_at: now_epoch(),
    };

    if db.is_none() {
        log_debug("refresh: IndexedDB unavailable; applying in-memory dataset only");
        emit_refresh(&mut on_progress, 100, "Using session-only cache");
        return Ok((
            prepared.packages,
            RefreshStatus::UpdatedInMemory {
                version: new_meta.version,
                package_count: new_meta.package_count,
                reason: "IndexedDB unavailable".to_string(),
            },
        ));
    }

    emit_refresh(&mut on_progress, 45, "Writing packages to local cache...");

    if let Err(initial_err) = replace_cache_with_progress(&prepared.packages, &new_meta, &mut on_progress).await {
        log_debug(&format!("refresh: cache replace failed: {initial_err}"));

        emit_refresh(
            &mut on_progress,
            56,
            "Local cache write failed, attempting repair...",
        );
        let recovered = attempt_db_reset_recovery_with_progress(&prepared.packages, &new_meta, &mut on_progress).await;
        if let Err(recovery_err) = recovered {
            let combined = format!("{initial_err}; recovery failed: {recovery_err}");
            let reason = if is_probable_storage_unavailable_text(&combined) {
                "IndexedDB writes unavailable in this browser profile".to_string()
            } else {
                format!("persistent cache unavailable: {combined}")
            };
            emit_refresh(&mut on_progress, 100, "Using session-only cache");
            return Ok((
                prepared.packages,
                RefreshStatus::UpdatedInMemory {
                    version: new_meta.version,
                    package_count: new_meta.package_count,
                    reason,
                },
            ));
        }
    }

    if ENABLE_STALE_PRUNE_ON_REFRESH {
        if let Err(err) = prune_stale_packages_with_progress(&prepared.packages, &mut on_progress).await {
            // Pruning is best-effort: keep refresh successful even if cleanup fails.
            log_debug(&format!("refresh: stale-key pruning skipped due to error: {err}"));
        }
    } else {
        log_debug("refresh: stale-key pruning disabled on foreground refresh path");
    }

    log_debug("refresh: cache replace succeeded");
    emit_refresh(&mut on_progress, 100, "Refresh complete");

    Ok((prepared.packages, RefreshStatus::Updated(new_meta)))
}

pub async fn run_storage_diagnostics() -> StorageDiagnosticsReport {
    let mut report = StorageDiagnosticsReport {
        current_origin: current_origin_label(),
        secure_context: current_secure_context(),
        storage_manager_available: false,
        persisted: None,
        persist_granted: None,
        estimate_usage_bytes: None,
        estimate_quota_bytes: None,
        indexeddb_package_entries: None,
        indexeddb_meta_entries: None,
        local_storage_entries: None,
        session_storage_entries: None,
        cache_storage_entries: None,
        indexeddb_write_ok: false,
        indexeddb_error: None,
        notes: Vec::new(),
    };

    if let Some(storage_obj) = get_storage_manager_object() {
        report.storage_manager_available = true;

        match call_storage_bool_method(&storage_obj, "persisted").await {
            Ok(Some(value)) => report.persisted = Some(value),
            Ok(None) => report.notes.push("storage.persisted() not available".to_string()),
            Err(err) => report
                .notes
                .push(format!("storage.persisted() failed: {err}")),
        }

        // Avoid calling persist() automatically: on some browsers/profiles this can
        // trigger interactive permission flows and stall diagnostics completion.
        report
            .notes
            .push("storage.persist() request skipped in diagnostics to avoid permission prompt stalls".to_string());

        match call_storage_estimate(&storage_obj).await {
            Ok((usage, quota)) => {
                report.estimate_usage_bytes = usage;
                report.estimate_quota_bytes = quota;
                if usage.is_none() || quota.is_none() {
                    report
                        .notes
                        .push("storage.estimate() returned partial values".to_string());
                }
            }
            Err(err) => report
                .notes
                .push(format!("storage.estimate() failed: {err}")),
        }
    } else {
        report
            .notes
            .push("navigator.storage unavailable on this runtime".to_string());
    }

    match probe_indexeddb_write().await {
        Ok(()) => report.indexeddb_write_ok = true,
        Err(err) => {
            report.indexeddb_error = Some(err.to_string());
            report
                .notes
                .push("IndexedDB write probe failed; app will stay session-only".to_string());
        }
    }

    match read_indexeddb_counts().await {
        Ok((package_entries, meta_entries)) => {
            report.indexeddb_package_entries = Some(package_entries);
            report.indexeddb_meta_entries = Some(meta_entries);

            if package_entries == 0 {
                report.notes.push(
                    "IndexedDB package store is empty; storage.estimate() may still show origin-wide retained usage"
                        .to_string(),
                );
            }
        }
        Err(err) => {
            report
                .notes
                .push(format!("IndexedDB entry count probe failed: {err}"));
        }
    }

    match read_other_storage_surface_counts().await {
        Ok(other) => {
            report.local_storage_entries = other.local_storage_entries;
            report.session_storage_entries = other.session_storage_entries;
            report.cache_storage_entries = other.cache_storage_entries;
            report.notes.extend(other.notes);
        }
        Err(err) => {
            report
                .notes
                .push(format!("Other storage surface probe failed: {err}"));
        }
    }

    report
}

pub async fn reset_local_cache() -> Result<(), CacheError> {
    // Prefer explicit store clears; deleting the whole DB can be blocked while
    // another connection is open.
    let db = open_db().await?;

    let tx = db
        .transaction(&[STORE_PACKAGES, STORE_META], TransactionMode::ReadWrite)
        .map_err(|e| CacheError::DbTx(format!("open reset tx failed: {e}")))?;
    let package_store = tx
        .store(STORE_PACKAGES)
        .map_err(|e| CacheError::DbTx(format!("open package store for reset failed: {e}")))?;
    let meta_store = tx
        .store(STORE_META)
        .map_err(|e| CacheError::DbTx(format!("open meta store for reset failed: {e}")))?;

    package_store
        .clear()
        .await
        .map_err(|e| CacheError::DbTx(format!("clear package store failed: {e}")))?;
    meta_store
        .clear()
        .await
        .map_err(|e| CacheError::DbTx(format!("clear meta store failed: {e}")))?;

    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit reset tx failed: {e}")))?;

    let (package_entries, meta_entries) = read_indexeddb_counts().await?;
    if package_entries != 0 || meta_entries != 0 {
        return Err(CacheError::DbTx(format!(
            "reset verification failed: packages={package_entries} meta={meta_entries}"
        )));
    }

    clear_other_storage_surfaces().await?;

    Ok(())
}

#[derive(Default)]
struct OtherStorageSurfaceCounts {
    local_storage_entries: Option<usize>,
    session_storage_entries: Option<usize>,
    cache_storage_entries: Option<usize>,
    notes: Vec<String>,
}

async fn read_other_storage_surface_counts() -> Result<OtherStorageSurfaceCounts, CacheError> {
    let mut out = OtherStorageSurfaceCounts::default();

    match count_window_storage_entries("localStorage").await {
        Ok(v) => out.local_storage_entries = Some(v),
        Err(err) => out.notes.push(format!("localStorage probe: {err}")),
    }
    match count_window_storage_entries("sessionStorage").await {
        Ok(v) => out.session_storage_entries = Some(v),
        Err(err) => out.notes.push(format!("sessionStorage probe: {err}")),
    }
    match count_cache_storage_entries().await {
        Ok(v) => out.cache_storage_entries = Some(v),
        Err(err) => out.notes.push(format!("CacheStorage probe: {err}")),
    }

    Ok(out)
}

async fn clear_other_storage_surfaces() -> Result<(), CacheError> {
    clear_window_storage("localStorage")
        .await
        .map_err(CacheError::DbTx)?;
    clear_window_storage("sessionStorage")
        .await
        .map_err(CacheError::DbTx)?;
    clear_cache_storage_entries().await.map_err(CacheError::DbTx)?;
    Ok(())
}

async fn count_window_storage_entries(storage_name: &str) -> Result<usize, String> {
    let storage = get_window_property(storage_name)?;
    if storage.is_null() || storage.is_undefined() {
        return Ok(0);
    }
    Reflect::get(&storage, &JsValue::from_str("length"))
        .map_err(|e| format!("{storage_name}.length read failed: {}", js_value_to_string(&e)))?
        .as_f64()
        .map(|v| v as usize)
        .ok_or_else(|| format!("{storage_name}.length is not numeric"))
}

async fn clear_window_storage(storage_name: &str) -> Result<(), String> {
    let storage = get_window_property(storage_name)?;
    if storage.is_null() || storage.is_undefined() {
        return Ok(());
    }

    let clear_fn = Reflect::get(&storage, &JsValue::from_str("clear"))
        .map_err(|e| format!("{storage_name}.clear read failed: {}", js_value_to_string(&e)))?
        .dyn_into::<Function>()
        .map_err(|_| format!("{storage_name}.clear is not callable"))?;

    clear_fn
        .call0(&storage)
        .map_err(|e| format!("{storage_name}.clear failed: {}", js_value_to_string(&e)))?;
    Ok(())
}

async fn count_cache_storage_entries() -> Result<usize, String> {
    let caches = get_window_property("caches")?;
    if caches.is_null() || caches.is_undefined() {
        return Ok(0);
    }

    let keys_value = call_method0_promise(&caches, "keys").await?;
    Ok(js_sys::Array::from(&keys_value).length() as usize)
}

async fn clear_cache_storage_entries() -> Result<usize, String> {
    let caches = get_window_property("caches")?;
    if caches.is_null() || caches.is_undefined() {
        return Ok(0);
    }

    let keys_value = call_method0_promise(&caches, "keys").await?;
    let keys = js_sys::Array::from(&keys_value);

    let mut deleted = 0usize;
    for key in keys.iter() {
        let deleted_value = call_method1_promise(&caches, "delete", &key).await?;
        if deleted_value.as_bool().unwrap_or(false) {
            deleted += 1;
        }
    }

    Ok(deleted)
}

fn get_window_property(name: &str) -> Result<JsValue, String> {
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_string())?;
    Reflect::get(&JsValue::from(window), &JsValue::from_str(name))
        .map_err(|e| format!("window.{name} read failed: {}", js_value_to_string(&e)))
}

async fn call_method0_promise(obj: &JsValue, method: &str) -> Result<JsValue, String> {
    let method_value = Reflect::get(obj, &JsValue::from_str(method))
        .map_err(|e| format!("{method} read failed: {}", js_value_to_string(&e)))?;
    let function = method_value
        .dyn_into::<Function>()
        .map_err(|_| format!("{method} is not callable"))?;
    let promise_value = function
        .call0(obj)
        .map_err(|e| format!("{method} call failed: {}", js_value_to_string(&e)))?;
    let promise = promise_value
        .dyn_into::<Promise>()
        .map_err(|_| format!("{method} did not return Promise"))?;
    JsFuture::from(promise)
        .await
        .map_err(|e| format!("{method} promise rejected: {}", js_value_to_string(&e)))
}

async fn call_method1_promise(obj: &JsValue, method: &str, arg: &JsValue) -> Result<JsValue, String> {
    let method_value = Reflect::get(obj, &JsValue::from_str(method))
        .map_err(|e| format!("{method} read failed: {}", js_value_to_string(&e)))?;
    let function = method_value
        .dyn_into::<Function>()
        .map_err(|_| format!("{method} is not callable"))?;
    let promise_value = function
        .call1(obj, arg)
        .map_err(|e| format!("{method} call failed: {}", js_value_to_string(&e)))?;
    let promise = promise_value
        .dyn_into::<Promise>()
        .map_err(|_| format!("{method} did not return Promise"))?;
    JsFuture::from(promise)
        .await
        .map_err(|e| format!("{method} promise rejected: {}", js_value_to_string(&e)))
}

async fn read_indexeddb_counts() -> Result<(usize, usize), CacheError> {
    let db = open_db().await?;
    let tx = db
        .transaction(&[STORE_PACKAGES, STORE_META], TransactionMode::ReadOnly)
        .map_err(|e| CacheError::DbTx(format!("open diagnostics count tx failed: {e}")))?;

    let packages_store = tx
        .store(STORE_PACKAGES)
        .map_err(|e| CacheError::DbTx(format!("open package count store failed: {e}")))?;
    let meta_store = tx
        .store(STORE_META)
        .map_err(|e| CacheError::DbTx(format!("open meta count store failed: {e}")))?;

    let package_entries = packages_store
        .get_all_keys(None, None)
        .await
        .map_err(|e| CacheError::DbTx(format!("count package keys failed: {e}")))?
        .len();
    let meta_entries = meta_store
        .get_all_keys(None, None)
        .await
        .map_err(|e| CacheError::DbTx(format!("count meta keys failed: {e}")))?
        .len();

    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit diagnostics count tx failed: {e}")))?;

    Ok((package_entries, meta_entries))
}

async fn open_db() -> Result<Rexie, CacheError> {
    let builder = Rexie::builder(DB_NAME)
        .version(DB_VERSION)
        .add_object_store(ObjectStore::new(STORE_PACKAGES).key_path("attr_path"))
        .add_object_store(ObjectStore::new(STORE_META).key_path("key"));

    builder
        .build()
        .await
        .map_err(|e| CacheError::OpenDb(e.to_string()))
}

async fn load_meta(db: &Rexie) -> Result<Option<CacheMeta>, CacheError> {
    let tx = db
        .transaction(&[STORE_META], TransactionMode::ReadOnly)
        .map_err(|e| CacheError::DbTx(e.to_string()))?;
    let store = tx
        .store(STORE_META)
        .map_err(|e| CacheError::DbTx(e.to_string()))?;

    let val = store
        .get(
            serde_wasm_bindgen::to_value(META_KEY_CURRENT)
                .map_err(|e| CacheError::Serde(e.to_string()))?,
        )
        .await
        .map_err(|e| CacheError::DbTx(e.to_string()))?;

    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit meta read tx failed: {e}")))?;

    match val {
        Some(v) => serde_wasm_bindgen::from_value(v)
            .map(Some)
            .map_err(|e| CacheError::Serde(e.to_string())),
        None => Ok(None),
    }
}

async fn load_packages(db: &Rexie) -> Result<Vec<Package>, CacheError> {
    let tx = db
        .transaction(&[STORE_PACKAGES], TransactionMode::ReadOnly)
        .map_err(|e| CacheError::DbTx(e.to_string()))?;
    let store = tx
        .store(STORE_PACKAGES)
        .map_err(|e| CacheError::DbTx(e.to_string()))?;
    let vals = store
        .get_all(None, None)
        .await
        .map_err(|e| CacheError::DbTx(e.to_string()))?;

    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit package read tx failed: {e}")))?;

    vals.into_iter()
        .map(|v| serde_wasm_bindgen::from_value(v).map_err(|e| CacheError::Serde(e.to_string())))
        .collect()
}

async fn replace_cache_with_progress<F>(
    packages: &[Package],
    meta: &CacheMeta,
    on_progress: &mut F,
) -> Result<(), CacheError>
where
    F: FnMut(RefreshProgress),
{
    const WRITE_CHUNK_SIZE: usize = 1500;
    const MAX_WRITE_ATTEMPTS: usize = 4;

    for attempt in 1..=MAX_WRITE_ATTEMPTS {
        match replace_cache_once_with_progress(packages, meta, WRITE_CHUNK_SIZE, on_progress).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                let is_retryable = matches!(&err, CacheError::DbTx(msg) if msg.contains("open package write tx failed") || msg.contains("commit package write tx failed") || msg.contains("open meta write tx failed"));
                if attempt == MAX_WRITE_ATTEMPTS || !is_retryable {
                    return Err(CacheError::DbTx(format!(
                        "replace cache failed after {attempt} attempt(s): {err}"
                    )));
                }

                let wait_ms = 40 * attempt as u32;
                log_debug(&format!(
                    "replace cache attempt {attempt} failed ({err}); retrying in {wait_ms}ms"
                ));
                emit_refresh(
                    on_progress,
                    58,
                    format!("Retrying local cache write ({attempt}/{MAX_WRITE_ATTEMPTS})..."),
                );
                TimeoutFuture::new(wait_ms).await;
            }
        }
    }

    Err(CacheError::DbTx(
        "replace cache ended unexpectedly without completion".to_string(),
    ))
}

async fn replace_cache_once_with_progress<F>(
    packages: &[Package],
    meta: &CacheMeta,
    write_chunk_size: usize,
    on_progress: &mut F,
) -> Result<(), CacheError>
where
    F: FnMut(RefreshProgress),
{
    let db = open_db().await?;

    // Use chunked upserts to avoid large single transactions and key-conflict failures.
    let total_chunks = packages.len().div_ceil(write_chunk_size).max(1);
    for (idx, chunk) in packages.chunks(write_chunk_size).enumerate() {
        let tx = db
            .transaction(&[STORE_PACKAGES], TransactionMode::ReadWrite)
            .map_err(|e| CacheError::DbTx(format!("open package write tx failed: {e}")))?;
        let package_store = tx
            .store(STORE_PACKAGES)
            .map_err(|e| CacheError::DbTx(format!("open package store failed: {e}")))?;

        for pkg in chunk {
            let val =
                serde_wasm_bindgen::to_value(pkg).map_err(|e| CacheError::Serde(e.to_string()))?;
            package_store.put(&val, None).await.map_err(|e| {
                CacheError::DbTx(format!("write package '{}' failed: {e}", pkg.attr_path))
            })?;
        }

        tx.done()
            .await
            .map_err(|e| CacheError::DbTx(format!("commit package write tx failed: {e}")))?;

        let written_chunks = idx + 1;
        let frac = (written_chunks as f64 / total_chunks as f64).clamp(0.0, 1.0);
        let pct = (45.0 + frac * 40.0).round() as u8;
        emit_refresh(
            on_progress,
            pct,
            format!("Writing packages to local cache... {written_chunks}/{total_chunks}"),
        );
    }

    emit_refresh(on_progress, 88, "Writing cache metadata...");

    let tx = db
        .transaction(&[STORE_META], TransactionMode::ReadWrite)
        .map_err(|e| CacheError::DbTx(format!("open meta write tx failed: {e}")))?;
    let meta_store = tx
        .store(STORE_META)
        .map_err(|e| CacheError::DbTx(format!("open meta store failed: {e}")))?;
    meta_store
        .put(
            &serde_wasm_bindgen::to_value(meta).map_err(|e| CacheError::Serde(e.to_string()))?,
            None,
        )
        .await
        .map_err(|e| CacheError::DbTx(format!("write cache meta failed: {e}")))?;
    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit meta write tx failed: {e}")))?;

    Ok(())
}

async fn prune_stale_packages_with_progress<F>(
    packages: &[Package],
    on_progress: &mut F,
) -> Result<(), CacheError>
where
    F: FnMut(RefreshProgress),
{
    const DELETE_CHUNK_SIZE: usize = 2000;

    let db = open_db().await?;
    let current_keys: HashSet<String> = packages.iter().map(|p| p.attr_path.clone()).collect();

    emit_refresh(on_progress, 90, "Scanning local cache for stale entries...");

    let read_tx = db
        .transaction(&[STORE_PACKAGES], TransactionMode::ReadOnly)
        .map_err(|e| CacheError::DbTx(format!("open package key-scan tx failed: {e}")))?;
    let read_store = read_tx
        .store(STORE_PACKAGES)
        .map_err(|e| CacheError::DbTx(format!("open package key-scan store failed: {e}")))?;

    let all_keys = read_store
        .get_all_keys(None, None)
        .await
        .map_err(|e| CacheError::DbTx(format!("scan package keys failed: {e}")))?;

    read_tx
        .done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit package key-scan tx failed: {e}")))?;

    let stale_keys = all_keys
        .into_iter()
        .filter_map(|key| key.as_string())
        .filter(|key| !current_keys.contains(key))
        .collect::<Vec<_>>();

    if stale_keys.is_empty() {
        emit_refresh(on_progress, 97, "No stale entries to prune");
        return Ok(());
    }

    log_debug(&format!(
        "refresh: pruning {} stale package key(s)",
        stale_keys.len()
    ));

    let mut deleted_total = 0usize;
    for chunk in stale_keys.chunks(DELETE_CHUNK_SIZE) {
        let write_tx = db
            .transaction(&[STORE_PACKAGES], TransactionMode::ReadWrite)
            .map_err(|e| CacheError::DbTx(format!("open package prune tx failed: {e}")))?;
        let write_store = write_tx
            .store(STORE_PACKAGES)
            .map_err(|e| CacheError::DbTx(format!("open package prune store failed: {e}")))?;

        for key in chunk {
            write_store
                .delete(serde_wasm_bindgen::to_value(key).map_err(|e| CacheError::Serde(e.to_string()))?)
                .await
                .map_err(|e| CacheError::DbTx(format!("delete stale package '{key}' failed: {e}")))?;
        }

        write_tx
            .done()
            .await
            .map_err(|e| CacheError::DbTx(format!("commit package prune tx failed: {e}")))?;

        deleted_total += chunk.len();
        let pct = (97.0 + (deleted_total as f64 / stale_keys.len() as f64) * 2.0).round() as u8;
        emit_refresh(on_progress, pct, "Pruning stale cache entries...");
    }

    Ok(())
}

async fn fetch_remote_manifest() -> Result<(RemoteManifest, String), CacheError> {
    let mut errors = Vec::new();

    for url in REMOTE_MANIFEST_CANDIDATES {
        let resp = match Request::get(url).send().await {
            Ok(r) => r,
            Err(err) => {
                errors.push(format!("{url}: network error {err}"));
                continue;
            }
        };

        if !resp.ok() {
            errors.push(format!("{url}: HTTP {}", resp.status()));
            continue;
        }

        let body = match resp.text().await {
            Ok(t) => t,
            Err(err) => {
                errors.push(format!("{url}: failed reading body: {err}"));
                continue;
            }
        };

        match serde_json::from_str::<RemoteManifest>(&body) {
            Ok(manifest) => {
                log_debug(&format!("manifest resolved from '{url}'"));
                return Ok((manifest, (*url).to_string()));
            }
            Err(err) => {
                let head = body.chars().take(80).collect::<String>();
                errors.push(format!(
                    "{url}: parse error {err}; body prefix='{}'",
                    head.replace('\n', "\\n")
                ));
            }
        }
    }

    Err(CacheError::Serde(format!(
        "manifest lookup failed: {}",
        errors.join(" | ")
    )))
}

#[derive(Debug, Clone)]
struct ArtifactCandidate {
    url: String,
    compressed: bool,
}

fn artifact_candidates(manifest: &RemoteManifest, manifest_source: &str) -> Vec<ArtifactCandidate> {
    let mut out = Vec::new();

    if manifest
        .compressed_format
        .as_deref()
        .map(|f| f.eq_ignore_ascii_case("brotli"))
        .unwrap_or(false)
    {
        if let Some(compressed_artifact) = manifest.compressed_artifact.as_deref() {
            out.push(ArtifactCandidate {
                url: resolve_artifact_url(manifest_source, compressed_artifact),
                compressed: true,
            });
        }
    }

    out.push(ArtifactCandidate {
        url: resolve_artifact_url(manifest_source, &manifest.artifact),
        compressed: false,
    });

    out
}

async fn fetch_artifact(candidate: &ArtifactCandidate) -> Result<PreparedData, CacheError> {
    let resp = Request::get(&candidate.url)
        .send()
        .await
        .map_err(|e| CacheError::Network(e.to_string()))?;

    if !resp.ok() {
        return Err(CacheError::Network(format!(
            "artifact HTTP status {}",
            resp.status()
        )));
    }

    if candidate.compressed {
        let bytes = resp
            .binary()
            .await
            .map_err(|e| CacheError::Serde(format!("failed to read compressed bytes: {e}")))?;
        let mut decompressor = brotli::Decompressor::new(bytes.as_slice(), 4096);
        let mut text = String::new();
        decompressor
            .read_to_string(&mut text)
            .map_err(|e| CacheError::Serde(format!("brotli decode failed: {e}")))?;
        serde_json::from_str::<PreparedData>(&text)
            .map_err(|e| CacheError::Serde(format!("decompressed JSON parse failed: {e}")))
    } else {
        resp.json::<PreparedData>()
            .await
            .map_err(|e| CacheError::Serde(e.to_string()))
    }
}

fn now_epoch() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

fn resolve_artifact_url(manifest_url: &str, artifact: &str) -> String {
    if artifact.starts_with("http://") || artifact.starts_with("https://") {
        return artifact.to_string();
    }
    if artifact.starts_with('/') {
        return artifact.to_string();
    }

    match manifest_url.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => format!("{dir}/{artifact}"),
        _ => artifact.to_string(),
    }
}

fn get_storage_manager_object() -> Option<JsValue> {
    let window = web_sys::window()?;
    let navigator = window.navigator();
    let navigator_js = JsValue::from(navigator);
    let storage = Reflect::get(&navigator_js, &JsValue::from_str("storage")).ok()?;
    if storage.is_null() || storage.is_undefined() {
        None
    } else {
        Some(storage)
    }
}

fn current_origin_label() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "unknown origin".to_string())
}

fn current_secure_context() -> Option<bool> {
    let window = web_sys::window()?;
    let window_js = JsValue::from(window);
    Reflect::get(&window_js, &JsValue::from_str("isSecureContext"))
        .ok()
        .and_then(|v| v.as_bool())
}

async fn call_storage_bool_method(storage_obj: &JsValue, method: &str) -> Result<Option<bool>, String> {
    let method_value = Reflect::get(storage_obj, &JsValue::from_str(method)).map_err(|e| {
        format!("failed to read method '{method}': {}", js_value_to_string(&e))
    })?;

    if method_value.is_undefined() || method_value.is_null() {
        return Ok(None);
    }

    let function = method_value
        .dyn_into::<Function>()
        .map_err(|_| format!("'{method}' is not callable"))?;
    let promise_value = function
        .call0(storage_obj)
        .map_err(|e| format!("'{method}' call failed: {}", js_value_to_string(&e)))?;
    let promise = promise_value
        .dyn_into::<Promise>()
        .map_err(|_| format!("'{method}' did not return a Promise"))?;
    let resolved = JsFuture::from(promise)
        .await
        .map_err(|e| format!("'{method}' promise rejected: {}", js_value_to_string(&e)))?;

    Ok(resolved.as_bool())
}

async fn call_storage_estimate(storage_obj: &JsValue) -> Result<(Option<f64>, Option<f64>), String> {
    let method_value = Reflect::get(storage_obj, &JsValue::from_str("estimate")).map_err(|e| {
        format!("failed to read method 'estimate': {}", js_value_to_string(&e))
    })?;

    if method_value.is_undefined() || method_value.is_null() {
        return Ok((None, None));
    }

    let function = method_value
        .dyn_into::<Function>()
        .map_err(|_| "'estimate' is not callable".to_string())?;
    let promise_value = function
        .call0(storage_obj)
        .map_err(|e| format!("'estimate' call failed: {}", js_value_to_string(&e)))?;
    let promise = promise_value
        .dyn_into::<Promise>()
        .map_err(|_| "'estimate' did not return a Promise".to_string())?;
    let resolved = JsFuture::from(promise)
        .await
        .map_err(|e| format!("'estimate' promise rejected: {}", js_value_to_string(&e)))?;

    let usage = Reflect::get(&resolved, &JsValue::from_str("usage"))
        .ok()
        .and_then(|v| v.as_f64());
    let quota = Reflect::get(&resolved, &JsValue::from_str("quota"))
        .ok()
        .and_then(|v| v.as_f64());

    Ok((usage, quota))
}

async fn probe_indexeddb_write() -> Result<(), CacheError> {
    let db = open_db().await?;
    let tx = db
        .transaction(&[STORE_META], TransactionMode::ReadWrite)
        .map_err(|e| CacheError::DbTx(format!("open diagnostics write tx failed: {e}")))?;
    let store = tx
        .store(STORE_META)
        .map_err(|e| CacheError::DbTx(format!("open diagnostics store failed: {e}")))?;

    let probe_meta = CacheMeta {
        key: META_KEY_DIAGNOSTICS_PROBE.to_string(),
        version: "diag".to_string(),
        checksum: "diag".to_string(),
        package_count: 0,
        built_at: now_epoch(),
        artifact: "diagnostics".to_string(),
        fetched_at: now_epoch(),
    };

    store
        .put(
            &serde_wasm_bindgen::to_value(&probe_meta)
                .map_err(|e| CacheError::Serde(e.to_string()))?,
            None,
        )
        .await
        .map_err(|e| CacheError::DbTx(format!("diagnostics write probe failed: {e}")))?;

    store
        .delete(
            serde_wasm_bindgen::to_value(META_KEY_DIAGNOSTICS_PROBE)
                .map_err(|e| CacheError::Serde(e.to_string()))?,
        )
        .await
        .map_err(|e| CacheError::DbTx(format!("diagnostics cleanup delete failed: {e}")))?;

    tx.done()
        .await
        .map_err(|e| CacheError::DbTx(format!("commit diagnostics write tx failed: {e}")))?;

    Ok(())
}

fn js_value_to_string(value: &JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| format!("{value:?}"))
}

fn log_debug(message: &str) {
    console::log_1(&format!("[cache-sync] {message}").into());
}

async fn attempt_db_reset_recovery_with_progress<F>(
    packages: &[Package],
    meta: &CacheMeta,
    on_progress: &mut F,
) -> Result<(), CacheError>
where
    F: FnMut(RefreshProgress),
{
    log_debug("refresh: attempting IndexedDB reset recovery");
    emit_refresh(on_progress, 60, "Resetting local cache database...");

    reset_local_cache().await?;

    emit_refresh(on_progress, 66, "Retrying cache write after reset...");
    replace_cache_with_progress(packages, meta, on_progress).await.map_err(|e| {
        CacheError::DbTx(format!("replace cache after DB reset failed: {e}"))
    })?;

    log_debug("refresh: IndexedDB reset recovery succeeded");
    Ok(())
}

fn emit_refresh<F>(on_progress: &mut F, percent: u8, detail: impl Into<String>)
where
    F: FnMut(RefreshProgress),
{
    on_progress(RefreshProgress {
        percent: percent.min(100),
        detail: detail.into(),
    });
}

fn is_probable_storage_unavailable_text(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("idb error")
        || lower.contains("quota")
        || lower.contains("notallowed")
        || lower.contains("security")
        || lower.contains("transactioninactive")
}

#[cfg(test)]
mod tests {
    use super::{RemoteManifest, artifact_candidates};

    fn base_manifest() -> RemoteManifest {
        RemoteManifest {
            version: "v1".to_string(),
            checksum: "abc".to_string(),
            package_count: 1,
            built_at: 0,
            artifact: "packages-v1.json".to_string(),
            compressed_artifact: None,
            compressed_format: None,
            compressed_size_bytes: None,
        }
    }

    #[test]
    fn artifact_candidates_prefers_compressed_then_fallback() {
        let mut m = base_manifest();
        m.compressed_artifact = Some("packages-v1.json.br".to_string());
        m.compressed_format = Some("brotli".to_string());

        let candidates = artifact_candidates(&m, "data/manifest.json");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].url, "data/packages-v1.json.br");
        assert!(candidates[0].compressed);
        assert_eq!(candidates[1].url, "data/packages-v1.json");
        assert!(!candidates[1].compressed);
    }

    #[test]
    fn artifact_candidates_uses_plain_only_without_supported_compression() {
        let mut m = base_manifest();
        m.compressed_artifact = Some("packages-v1.json.br".to_string());
        m.compressed_format = Some("gzip".to_string());

        let candidates = artifact_candidates(&m, "data/manifest.json");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].url, "data/packages-v1.json");
        assert!(!candidates[0].compressed);
    }
}
