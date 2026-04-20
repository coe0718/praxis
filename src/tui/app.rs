use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::{
    events::read_events_since,
    heartbeat::read_heartbeat,
    paths::PraxisPaths,
    state::SessionState,
    storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore},
    tools::{FileToolRegistry, ToolRegistry},
};

const TICK_MS: u64 = 750;

struct TuiState {
    phase: String,
    goal: String,
    outcome: String,
    action: String,
    heartbeat_phase: String,
    heartbeat_time: String,
    tool_count: usize,
    pending_approvals: usize,
    recent_events: Vec<String>,
}

impl TuiState {
    fn load(paths: &PraxisPaths) -> Self {
        let mut state = TuiState {
            phase: "–".to_string(),
            goal: "–".to_string(),
            outcome: "–".to_string(),
            action: "–".to_string(),
            heartbeat_phase: "–".to_string(),
            heartbeat_time: "–".to_string(),
            tool_count: 0,
            pending_approvals: 0,
            recent_events: Vec::new(),
        };

        if let Ok(Some(session)) = SessionState::load(&paths.state_file) {
            state.phase = session.current_phase.to_string();
            state.outcome = session.last_outcome.unwrap_or_else(|| "–".to_string());
            state.goal = session
                .selected_goal_id
                .as_deref()
                .zip(session.selected_goal_title.as_deref())
                .map(|(id, title)| format!("{id}: {title}"))
                .unwrap_or_else(|| "–".to_string());
            state.action = session.action_summary.unwrap_or_else(|| "–".to_string());
        }

        if let Ok(hb) = read_heartbeat(&paths.heartbeat_file) {
            state.heartbeat_phase = hb.phase;
            state.heartbeat_time = hb.updated_at;
        }

        if let Ok(tools) = FileToolRegistry.list(paths) {
            state.tool_count = tools.len();
        }

        let store = SqliteSessionStore::new(paths.database_file.clone());
        if let Ok(approvals) = store.list_approvals(Some(ApprovalStatus::Pending)) {
            state.pending_approvals = approvals.len();
        }

        if let Ok((events, _)) = read_events_since(&paths.events_file, 0) {
            state.recent_events = events
                .into_iter()
                .rev()
                .take(12)
                .map(|e| format!("[{}] {}", e.kind, e.detail))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
        }

        state
    }
}

pub fn run_tui(data_dir: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let paths = PraxisPaths::for_data_dir(data_dir);
    let mut last_tick = Instant::now();

    loop {
        let state = TuiState::load(&paths);
        terminal.draw(|frame| render(frame, &state))?;

        let timeout = TICK_MS.saturating_sub(last_tick.elapsed().as_millis() as u64);
        if event::poll(Duration::from_millis(timeout))?
            && let Event::Key(key) = event::read()?
        {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                _ => {}
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render(frame: &mut Frame, state: &TuiState) {
    let area = frame.area();

    // Outer vertical split: header | body | footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Body horizontal split: left panel | events
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(outer[1]);

    // Left vertical split: status | action
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(body[0]);

    render_header(frame, outer[0]);
    render_status(frame, left[0], state);
    render_action(frame, left[1], state);
    render_events(frame, body[1], state);
    render_footer(frame, outer[2]);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Praxis ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("— live dashboard", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn render_status(frame: &mut Frame, area: Rect, state: &TuiState) {
    let phase_color = match state.phase.as_str() {
        "orient" => Color::Blue,
        "decide" => Color::Yellow,
        "act" => Color::Green,
        "reflect" => Color::Magenta,
        "sleep" => Color::DarkGray,
        _ => Color::White,
    };

    let items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled("phase     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &state.phase,
                Style::default()
                    .fg(phase_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("outcome   ", Style::default().fg(Color::DarkGray)),
            Span::raw(&state.outcome),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("goal      ", Style::default().fg(Color::DarkGray)),
            Span::raw(truncate(&state.goal, 36)),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("heartbeat ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!(
                "{} @ {}",
                state.heartbeat_phase, state.heartbeat_time
            )),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("tools     ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.tool_count.to_string()),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("queue     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} pending", state.pending_approvals),
                if state.pending_approvals > 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ])),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Status ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, area);
}

fn render_action(frame: &mut Frame, area: Rect, state: &TuiState) {
    let paragraph = Paragraph::new(state.action.as_str())
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Last Action ")
                .title_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(paragraph, area);
}

fn render_events(frame: &mut Frame, area: Rect, state: &TuiState) {
    let items: Vec<ListItem> = state
        .recent_events
        .iter()
        .map(|e| {
            let color = if e.contains("error") || e.contains("fail") {
                Color::Red
            } else if e.contains("complete") || e.contains("success") {
                Color::Green
            } else if e.contains("blocked") || e.contains("budget") {
                Color::Yellow
            } else {
                Color::White
            };
            ListItem::new(Span::styled(truncate(e, 60), Style::default().fg(color)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Recent Events ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Span::styled(
        " q / Ctrl-C to quit   refreshes every 750ms",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(footer, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}…",
            s.chars().take(max.saturating_sub(1)).collect::<String>()
        )
    }
}
