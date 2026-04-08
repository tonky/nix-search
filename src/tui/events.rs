use crossterm::event::{Event, KeyCode, KeyModifiers};

use super::msg::Msg;

pub fn to_msg(event: Event) -> anyhow::Result<Option<Msg>> {
    let msg = match event {
        Event::Key(key) => match (key.modifiers, key.code) {
            (_, KeyCode::Enter) => Some(Msg::Select),
            (_, KeyCode::Esc) => Some(Msg::Quit),
            (_, KeyCode::Up) => Some(Msg::MoveUp),
            (_, KeyCode::Down) => Some(Msg::MoveDown),
            (_, KeyCode::Tab) => Some(Msg::TogglePane),
            (_, KeyCode::Char('?')) => Some(Msg::ToggleHelp),
            (KeyModifiers::CONTROL, KeyCode::Char('o')) => Some(Msg::OpenHomepage),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Msg::QueryClear),
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => Some(Msg::TogglePlatform),
            (_, KeyCode::Backspace) => Some(Msg::QueryBackspace),
            (_, KeyCode::PageUp) => Some(Msg::ScrollDetailUp),
            (_, KeyCode::PageDown) => Some(Msg::ScrollDetailDown),
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                Some(Msg::QueryAppend(c))
            }
            _ => None,
        },
        Event::Resize(_, h) => {
            let rows = h.saturating_sub(2) as usize;
            Some(Msg::ViewportRowsChanged(rows))
        }
        _ => None,
    };

    Ok(msg)
}
