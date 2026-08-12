use std::path::Path;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::config::Host;

const ACCENT: Color = Color::Cyan;

pub fn render(frame: &mut Frame<'_>, app: &App, config_path: &Path) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, app, config_path, header_area);
    let [list_area, details_area] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .areas(body_area);
    render_hosts(frame, app, list_area);
    render_details(frame, app.selected_host(), details_area);
    render_footer(frame, app, footer_area);

    if app.is_showing_help() {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame<'_>, app: &App, config_path: &Path, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" lazyssh ", Style::default().fg(Color::Black).bg(ACCENT)),
        Span::raw(format!(
            "  {}/{} hosts",
            app.visible_count(),
            app.total_count()
        )),
    ]);
    let header = Paragraph::new(config_path.display().to_string())
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, area);
}

fn render_hosts(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let hosts = app.visible_hosts();
    let items = if hosts.is_empty() {
        vec![ListItem::new(if app.query().is_empty() {
            "  No concrete Host entries"
        } else {
            "  No matching hosts"
        })]
    } else {
        hosts
            .iter()
            .map(|host| {
                let target = host.host_name.as_deref().unwrap_or(&host.alias);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<20}", host.alias),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(target, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Hosts "))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default().with_selected(app.selected_index());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame<'_>, host: Option<&Host>, area: Rect) {
    let lines = host.map_or_else(
        || vec![Line::from("Select a host to inspect its SSH settings.")],
        |host| {
            let mut lines = vec![
                field("Alias", &host.alias),
                field("Destination", destination(host)),
                field("HostName", host.host_name.as_deref().unwrap_or("—")),
                field("User", host.user.as_deref().unwrap_or("—")),
                field(
                    "Port",
                    host.port
                        .map_or_else(|| "—".into(), |port| port.to_string()),
                ),
                field("ProxyJump", host.proxy_jump.as_deref().unwrap_or("—")),
            ];
            if host.identity_files.is_empty() {
                lines.push(field("IdentityFile", "—"));
            } else {
                for (index, identity) in host.identity_files.iter().enumerate() {
                    lines.push(field(
                        if index == 0 { "IdentityFile" } else { "" },
                        identity,
                    ));
                }
            }
            lines.push(Line::from(""));
            lines.push(field(
                "Source",
                format!("{}:{}", host.source.display(), host.line),
            ));
            lines
        },
    );

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Connection ")),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (text, style) = if app.is_searching() {
        (
            format!("Filter: /{}█", app.query()),
            Style::default().fg(ACCENT),
        )
    } else if let Some(status) = app.status() {
        (status.to_owned(), Style::default().fg(Color::Yellow))
    } else {
        (
            "↑↓/jk navigate  / filter  Enter connect  r reload  ? help  q quit".into(),
            Style::default().fg(Color::DarkGray),
        )
    };
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_help(frame: &mut Frame<'_>) {
    let area = centered_rect(54, 16, frame.area());
    frame.render_widget(Clear, area);
    let help = Paragraph::new(vec![
        Line::from(""),
        shortcut("↑ / k", "previous host"),
        shortcut("↓ / j", "next host"),
        shortcut("g / G", "first / last host"),
        shortcut("/", "filter hosts (fuzzy match)"),
        shortcut("Enter", "connect with system ssh"),
        shortcut("r", "reload SSH config"),
        shortcut("? / q", "close this help"),
        Line::from(""),
        Line::styled(
            "Read-only: lazyssh never modifies SSH config.",
            Style::default().fg(Color::DarkGray),
        ),
    ])
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .title(" Help "),
    );
    frame.render_widget(help, area);
}

fn field(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<13}"), Style::default().fg(ACCENT)),
        Span::raw(value.into()),
    ])
}

fn shortcut(key: &str, description: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<10}"), Style::default().fg(ACCENT)),
        Span::raw(description.to_owned()),
    ])
}

fn destination(host: &Host) -> String {
    let hostname = host.host_name.as_deref().unwrap_or(&host.alias);
    let mut destination = host
        .user
        .as_ref()
        .map_or_else(|| hostname.to_owned(), |user| format!("{user}@{hostname}"));
    if let Some(port) = host.port {
        destination.push(':');
        destination.push_str(&port.to_string());
    }
    destination
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(height.min(area.height))])
        .flex(Flex::Center)
        .areas(area);
    area
}
