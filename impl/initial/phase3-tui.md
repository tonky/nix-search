# Phase 3: TUI (Elm Architecture)

## Goal

Build an interactive two-pane TUI that uses Elm architecture (`Model`, `Msg`, `update`, `view`, `Cmd`) so state changes are deterministic and side effects are isolated.

`Enter` prints selected attr path to stdout and exits 0. TUI always renders on stderr.

## Prerequisites

Phase 1 + 2 complete. `search::search()` and `SearchResults` available.

## Deliverable

```bash
nix-search claude code              # opens TUI, navigate, Enter selects
flox install $(nix-search claude)   # TUI renders on terminal, attr path captured by $()
nix-search                          # opens TUI with blank query
Esc / q                             # exits 1 (cancel)
```

---

## New Dependencies

```toml
ratatui = "0.28"
crossterm = "0.28"
```

---

## Module Layout

```
src/
  tui/
    mod.rs      runtime loop + terminal setup/teardown
    model.rs    Model + Pane + helpers
    msg.rs      Msg enum (user + internal events)
    cmd.rs      Cmd enum + execute_all()
    update.rs   update(model, msg) -> Vec<Cmd>
    events.rs   crossterm Event -> Msg mapping
    view.rs     ratatui rendering (pure)
```

---

## Elm Runtime

### 1. `src/tui/model.rs`

```rust
pub struct Model {
    pub query: String,
    pub results: SearchResults,
    pub selected: usize,
    pub list_offset: usize,
    pub detail_scroll: usize,
    pub focus: Pane,
    pub platform: Option<String>,
    pub channel: String,
    pub total_packages: usize,

    pub should_quit: bool,
    pub selection: Option<String>,

    pub internal_tx: std::sync::mpsc::Sender<Msg>,
    pub internal_rx: std::sync::mpsc::Receiver<Msg>,
}

pub enum Pane { List, Detail }
```

### 2. `src/tui/msg.rs`

```rust
pub enum Msg {
    Tick,
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

    ToggleHelp,
    Resize(u16, u16),

    SearchCompleted(SearchResults),
    SearchFailed(String),
}
```

### 3. `src/tui/cmd.rs`

```rust
pub enum Cmd {
    RunSearch { query: String, platform: Option<String>, limit: usize },
    OpenHomepage,
    Noop,
}

pub fn execute_all(model: &mut Model, cmds: Vec<Cmd>) -> anyhow::Result<()> {
    for cmd in cmds {
        execute(model, cmd)?;
    }
    Ok(())
}
```

`execute()` performs side effects only, and sends completion messages back through `internal_tx`.

### 4. `src/tui/update.rs`

```rust
pub fn update(model: &mut Model, msg: Msg) -> anyhow::Result<Vec<Cmd>>
```

Rules:
- Keep this function pure except model mutation.
- Never do I/O inside `update()`.
- Returning `Cmd::RunSearch` is the only way search is triggered.

Examples:
- `Msg::QueryAppend(c)` mutates query, resets selection, returns `RunSearch`.
- `Msg::TogglePlatform` mutates platform, resets selection, returns `RunSearch`.
- `Msg::Select` sets `selection`, `should_quit = true`, returns no commands.

### 5. `src/tui/events.rs`

```rust
pub fn to_msg(event: crossterm::event::Event) -> anyhow::Result<Option<Msg>>
```

Key mappings:
- `Up` => `MoveUp`
- `Down` => `MoveDown`
- `Enter` => `Select`
- `Esc` or `q` => `Quit`
- `Tab` => `TogglePane`
- `Ctrl+U` => `QueryClear`
- `Ctrl+P` => `TogglePlatform`
- printable chars => `QueryAppend(c)`
- `Backspace` => `QueryBackspace`
- `?` => `ToggleHelp`

### 6. `src/tui/view.rs`

```rust
pub fn render(frame: &mut ratatui::Frame, model: &Model)
```

The `view` is pure: it renders based only on `Model`.

### 7. `src/tui/mod.rs`

Runtime loop:

```rust
loop {
    terminal.draw(|f| view::render(f, &model))?;

    while let Ok(msg) = model.internal_rx.try_recv() {
        let cmds = update::update(&mut model, msg)?;
        cmd::execute_all(&mut model, cmds)?;
    }

    if event::poll(Duration::from_millis(16))? {
        let ev = event::read()?;
        if let Some(msg) = events::to_msg(ev)? {
            let cmds = update::update(&mut model, msg)?;
            cmd::execute_all(&mut model, cmds)?;
        }
    }

    if model.should_quit {
        cleanup()?;
        return Ok(model.selection.take());
    }
}
```

---

## TTY Dispatch

In `main.rs`:
- Launch TUI only when `stdout` is a TTY and no non-interactive output mode is selected.
- Condition: `is_tty && !cli.json && !cli.plain && !cli.first`.

---

## Definition of Done

- [ ] Elm architecture modules (`Model`, `Msg`, `Cmd`, `update`, `view`) exist and are wired
- [ ] Typing and navigation work with predictable state transitions
- [ ] `Enter` exits with selected attr to stdout
- [ ] `Esc`/`q` exits 1 without printing
- [ ] `Ctrl+P` toggles platform filter via `Msg` + `Cmd::RunSearch`
- [ ] `Tab` switches pane focus
- [ ] TUI renders on stderr, selected value prints on stdout
- [ ] Terminal cleanup is reliable on every exit path
