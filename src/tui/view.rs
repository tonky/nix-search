use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use super::model::Model;

pub fn render(frame: &mut ratatui::Frame, model: &Model) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    let header = render_header(model);
    frame.render_widget(header, chunks[0]);

    let (list, detail) = render_main(model, main[0].height as usize);
    frame.render_widget(list, main[0]);
    frame.render_widget(detail, main[1]);

    let footer = Paragraph::new(
        "Up/Down navigate  Enter select  Tab pane  ^P platform  ^U clear  ^O open  Esc quit",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);

    if model.show_help {
        render_help(frame);
    }
}

fn render_header(model: &Model) -> Paragraph<'_> {
    let mut right = format!("[{}", model.channel);
    if let Some(p) = &model.platform {
        right.push_str(&format!(" | {}", p));
    } else {
        right.push_str(" | all-platforms");
    }
    right.push(']');
    right.push_str(&format!("  {}", model.flat_len()));

    if model.cache_refreshing {
        let spinner = ["|", "/", "-", "\\\\"];
        let s = spinner[(model.tick as usize) % spinner.len()];
        right.push(' ');
        right.push_str(s);
    }

    Paragraph::new(Line::from(vec![
        Span::raw(format!("> {}_", model.query)),
        Span::raw(" "),
        Span::styled(right, Style::default().fg(Color::DarkGray)),
    ]))
}

fn render_main(model: &Model, list_height: usize) -> (List<'_>, Paragraph<'_>) {
    let visible_rows = list_height.max(1);

    let total_results = model.flat_len();
    let separator_at = model.platform_separator_at();
    let total_rows = total_results + usize::from(separator_at.is_some());
    let end_row = visible_rows.min(total_rows);

    let mut items = Vec::new();
    for row in 0..end_row {
        if separator_at == Some(row) {
            items.push(
                ListItem::new("-- other platforms --").style(Style::default().fg(Color::DarkGray)),
            );
            continue;
        }

        let Some(i) = row_to_result_index(row, separator_at) else {
            continue;
        };
        let Some(sp) = model.result_at(i) else {
            continue;
        };

        let mut style = Style::default();
        if i == model.selected {
            style = style
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD);
        }

        let row = format!("{:<28} {:>12}", sp.package.attr_path, sp.package.version);
        items.push(ListItem::new(row).style(style));
    }

    let list = List::new(items).block(Block::default().borders(Borders::RIGHT));

    let detail = match model.result_at(model.selected) {
        None => Paragraph::new("no selection"),
        Some(sp) => {
            let p = &sp.package;
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("attr: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&p.attr_path),
                ]),
                Line::from(vec![
                    Span::styled("version: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&p.version),
                ]),
                Line::from(vec![
                    Span::styled("score: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!("{:.2}", sp.score)),
                ]),
                Line::raw(""),
                Line::from(vec![Span::styled(
                    "desc: ",
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
                Line::raw(p.description.clone()),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("platforms: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(p.platforms.join(", ")),
                ]),
            ];

            if model.enriched_loading {
                lines.push(Line::raw(""));
                lines.push(Line::raw("loading enrichment..."));
            } else if let Some(details) = &model.enriched {
                if let Some(homepage) = details.homepage.first() {
                    lines.push(Line::raw(""));
                    lines.push(Line::from(vec![
                        Span::styled("homepage: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(homepage.clone()),
                    ]));
                }
                if !details.license.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("license: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(details.license.join(", ")),
                    ]));
                }
                if !details.maintainers.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "maintainers: ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(details.maintainers.join(" ")),
                    ]));
                }
                if details.broken {
                    lines.push(Line::from(vec![
                        Span::styled("broken: ", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(
                            "YES",
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                }
            }

            if let Some(err) = &model.detail_error {
                lines.push(Line::raw(""));
                lines.push(Line::styled(err.clone(), Style::default().fg(Color::Red)));
            }

            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: true })
                .scroll((model.detail_scroll as u16, 0))
        }
    };

    (list, detail)
}

fn row_to_result_index(row: usize, separator_at: Option<usize>) -> Option<usize> {
    match separator_at {
        Some(sep) if row == sep => None,
        Some(sep) if row > sep => Some(row - 1),
        _ => Some(row),
    }
}

fn render_help(frame: &mut ratatui::Frame) {
    let area = centered_rect(60, 50, frame.area());
    let lines = vec![
        Line::from("nix-search keyboard shortcuts"),
        Line::raw(""),
        Line::from("Up/Down              navigate"),
        Line::from("Enter                select"),
        Line::from("Tab                  switch pane"),
        Line::from("Ctrl+U               clear query"),
        Line::from("Ctrl+P               toggle platform"),
        Line::from("Ctrl+O               open homepage"),
        Line::from("Esc                  quit"),
        Line::from("?                    close help"),
    ];
    let popup = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" help "))
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
