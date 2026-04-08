use std::io::{BufWriter, stderr};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossterm::ExecutableCommand;
use crossterm::event;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::cache;
use crate::cache::index::NixIndex;
mod cmd;
mod events;
mod model;
mod msg;
mod update;
mod view;

use cmd::{Cmd, WorkerTask, execute_all, spawn_worker};
use model::Model;
use msg::Msg;

pub fn run_tui(
    nix_index: &NixIndex,
    initial_query: &str,
    platform: Option<String>,
    channel: &str,
    ttl: u64,
) -> anyhow::Result<Option<String>> {
    let cache_dir = dirs::cache_dir()
        .map(|p| p.join("nix-search"))
        .unwrap_or_else(|| PathBuf::from(".cache/nix-search"));

    let (internal_tx, internal_rx) = std::sync::mpsc::channel::<Msg>();
    let worker_tx = spawn_worker(internal_tx.clone());

    let mut model = Model::new(
        initial_query,
        platform,
        channel,
        cache_dir.clone(),
        internal_tx,
        internal_rx,
        worker_tx.clone(),
    );

    enable_raw_mode()?;
    let mut err = BufWriter::new(stderr());
    err.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(&mut err);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let _ = model.set_list_viewport_rows(rows_from_terminal_height(size.height));

    let initial_cmds = vec![
        Cmd::RunSearch {
            query: model.query.clone(),
            platform: model.platform.clone(),
            limit: model.search_limit(),
            exact_attr: None,
        },
        Cmd::ResolveEsConfig,
    ];
    execute_all(&mut model, nix_index, initial_cmds)?;

    if is_stale(&cache_dir, channel, ttl) {
        model.cache_refreshing = true;
        worker_tx
            .send(WorkerTask::RefreshCache {
                cache_dir: cache_dir.clone(),
                channel: channel.to_string(),
            })
            .ok();
    }

    let result = loop {
        terminal.draw(|frame| view::render(frame, &model))?;

        while let Ok(msg) = model.internal_rx.try_recv() {
            let cmds = update::update(&mut model, msg)?;
            execute_all(&mut model, nix_index, cmds)?;
        }

        if event::poll(Duration::from_millis(16))? {
            let ev = event::read()?;
            if let Some(msg) = events::to_msg(ev)? {
                let cmds = update::update(&mut model, msg)?;
                execute_all(&mut model, nix_index, cmds)?;
            }
        } else {
            model.tick = model.tick.wrapping_add(1);
        }

        if model.should_quit {
            break model.selection.take();
        }
    };

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(result)
}

fn is_stale(cache_dir: &std::path::Path, channel: &str, ttl: u64) -> bool {
    let Some(meta) = cache::load_meta(cache_dir, channel) else {
        return true;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(meta.fetched_at) > ttl
}

fn rows_from_terminal_height(height: u16) -> usize {
    // One header row and one footer row are reserved.
    height.saturating_sub(2) as usize
}
