/// # ratatui-showcase
///
/// An interactive TUI demonstrating ratatui's widgets, layouts, and patterns.
/// Navigate between tabs to explore different widget types.
///
/// Controls:
///   Tab / Shift+Tab  — switch tabs
///   ↑/↓ or j/k       — navigate lists/tables
///   Enter             — toggle/select items
///   a                 — add item (in Todo tab)
///   d/Delete          — delete item (in Todo tab)
///   +/-               — adjust gauge values
///   q / Esc           — quit
use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Dataset, Chart, Axis, GraphType,
        Gauge, LineGauge, List, ListItem, ListState,
        Paragraph, Row,
        Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};

// ============================================================
// App State
// ============================================================

struct App {
    /// Currently selected tab
    current_tab: usize,
    tab_titles: Vec<&'static str>,

    /// Todo list state (Tab 0)
    todos: Vec<TodoItem>,
    todo_state: ListState,
    adding_todo: bool,
    todo_input: String,

    /// Table state (Tab 1)
    table_data: Vec<Vec<String>>,
    table_state: TableState,

    /// Gauges (Tab 2)
    cpu_usage: f64,
    memory_usage: f64,
    disk_usage: f64,
    download_progress: f64,

    /// Chart data (Tab 3)
    chart_data: Vec<(f64, f64)>,
    sparkline_data: Vec<u64>,
    tick_count: u64,

    /// Should quit
    should_quit: bool,

    /// Start time for animations
    start_time: Instant,

    /// Popup visible
    show_help: bool,
}

struct TodoItem {
    text: String,
    done: bool,
}

impl App {
    fn new() -> Self {
        let mut todo_state = ListState::default();
        todo_state.select(Some(0));

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        App {
            current_tab: 0,
            tab_titles: vec!["📝 Todo", "📊 Table", "🔋 Gauges", "📈 Charts", "📖 About"],

            todos: vec![
                TodoItem { text: "Learn ratatui basics".into(), done: true },
                TodoItem { text: "Build a layout with constraints".into(), done: true },
                TodoItem { text: "Add keyboard navigation".into(), done: false },
                TodoItem { text: "Style widgets with colors".into(), done: false },
                TodoItem { text: "Create a stateful list".into(), done: false },
                TodoItem { text: "Handle events in a loop".into(), done: false },
                TodoItem { text: "Deploy to production 🚀".into(), done: false },
            ],
            todo_state,
            adding_todo: false,
            todo_input: String::new(),

            table_data: vec![
                vec!["ratatui".into(), "0.29".into(), "TUI framework".into(), "★★★★★".into()],
                vec!["crossterm".into(), "0.28".into(), "Terminal backend".into(), "★★★★★".into()],
                vec!["clap".into(), "4.5".into(), "CLI argument parser".into(), "★★★★★".into()],
                vec!["serde".into(), "1.0".into(), "Serialization".into(), "★★★★☆".into()],
                vec!["tokio".into(), "1.0".into(), "Async runtime".into(), "★★★★★".into()],
                vec!["reqwest".into(), "0.12".into(), "HTTP client".into(), "★★★★☆".into()],
                vec!["anyhow".into(), "1.0".into(), "Error handling".into(), "★★★★☆".into()],
                vec!["sha2".into(), "0.10".into(), "SHA-2 hashing".into(), "★★★☆☆".into()],
                vec!["indicatif".into(), "0.17".into(), "Progress bars".into(), "★★★★☆".into()],
                vec!["colored".into(), "2.0".into(), "Terminal colors".into(), "★★★☆☆".into()],
            ],
            table_state,

            cpu_usage: 0.42,
            memory_usage: 0.67,
            disk_usage: 0.31,
            download_progress: 0.0,

            chart_data: Vec::new(),
            sparkline_data: vec![0; 60],
            tick_count: 0,

            should_quit: false,
            start_time: Instant::now(),
            show_help: false,
        }
    }

    fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tab_titles.len();
    }

    fn prev_tab(&mut self) {
        if self.current_tab == 0 {
            self.current_tab = self.tab_titles.len() - 1;
        } else {
            self.current_tab -= 1;
        }
    }

    fn tick(&mut self) {
        self.tick_count += 1;

        // Animate download progress
        if self.download_progress < 1.0 {
            self.download_progress += 0.005;
            if self.download_progress > 1.0 {
                self.download_progress = 1.0;
            }
        }

        // Simulate fluctuating CPU
        let t = self.start_time.elapsed().as_secs_f64();
        self.cpu_usage = 0.3 + 0.2 * (t * 0.5).sin() + 0.1 * (t * 1.3).cos();
        self.cpu_usage = self.cpu_usage.clamp(0.0, 1.0);

        // Update chart data (sine wave)
        self.chart_data.clear();
        for i in 0..100 {
            let x = i as f64 * 0.1;
            let y = (x + t * 0.5).sin() * 2.0 + (x * 2.0 + t).cos();
            self.chart_data.push((x, y));
        }

        // Update sparkline
        let val = ((self.cpu_usage * 100.0) as u64).clamp(0, 100);
        self.sparkline_data.push(val);
        if self.sparkline_data.len() > 60 {
            self.sparkline_data.remove(0);
        }
    }

    // -- Todo actions --

    fn todo_next(&mut self) {
        let len = self.todos.len();
        if len == 0 { return; }
        let i = self.todo_state.selected().unwrap_or(0);
        self.todo_state.select(Some((i + 1) % len));
    }

    fn todo_prev(&mut self) {
        let len = self.todos.len();
        if len == 0 { return; }
        let i = self.todo_state.selected().unwrap_or(0);
        self.todo_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
    }

    fn todo_toggle(&mut self) {
        if let Some(i) = self.todo_state.selected() {
            if i < self.todos.len() {
                self.todos[i].done = !self.todos[i].done;
            }
        }
    }

    fn todo_delete(&mut self) {
        if let Some(i) = self.todo_state.selected() {
            if i < self.todos.len() {
                self.todos.remove(i);
                if !self.todos.is_empty() {
                    let new_idx = if i >= self.todos.len() { self.todos.len() - 1 } else { i };
                    self.todo_state.select(Some(new_idx));
                } else {
                    self.todo_state.select(None);
                }
            }
        }
    }

    fn todo_add(&mut self) {
        if !self.todo_input.is_empty() {
            self.todos.push(TodoItem {
                text: self.todo_input.drain(..).collect(),
                done: false,
            });
            self.todo_state.select(Some(self.todos.len() - 1));
        }
        self.adding_todo = false;
    }

    // -- Table actions --

    fn table_next(&mut self) {
        let len = self.table_data.len();
        if len == 0 { return; }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some((i + 1) % len));
    }

    fn table_prev(&mut self) {
        let len = self.table_data.len();
        if len == 0 { return; }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
    }
}

// ============================================================
// Main — setup terminal, run event loop, restore terminal
// ============================================================

fn main() -> io::Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run
    let result = run_app(&mut terminal);

    // Restore
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        // Draw
        terminal.draw(|f| draw(f, &mut app))?;

        // Handle events with timeout
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // If adding a todo, handle text input
                if app.adding_todo {
                    match key.code {
                        KeyCode::Enter => app.todo_add(),
                        KeyCode::Esc => {
                            app.adding_todo = false;
                            app.todo_input.clear();
                        }
                        KeyCode::Backspace => { app.todo_input.pop(); }
                        KeyCode::Char(c) => app.todo_input.push(c),
                        _ => {}
                    }
                    continue;
                }

                // Help popup
                if app.show_help {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                            app.show_help = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Global keys
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Tab => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.prev_tab();
                        } else {
                            app.next_tab();
                        }
                    }
                    KeyCode::Char('?') => app.show_help = true,
                    _ => {
                        // Tab-specific keys
                        match app.current_tab {
                            0 => handle_todo_keys(&mut app, key.code),
                            1 => handle_table_keys(&mut app, key.code),
                            2 => handle_gauge_keys(&mut app, key.code),
                            _ => {}
                        }
                    }
                }
            }
        }

        // Tick for animations
        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_todo_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Down | KeyCode::Char('j') => app.todo_next(),
        KeyCode::Up | KeyCode::Char('k') => app.todo_prev(),
        KeyCode::Enter | KeyCode::Char(' ') => app.todo_toggle(),
        KeyCode::Char('a') => {
            app.adding_todo = true;
            app.todo_input.clear();
        }
        KeyCode::Char('d') | KeyCode::Delete => app.todo_delete(),
        _ => {}
    }
}

fn handle_table_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Down | KeyCode::Char('j') => app.table_next(),
        KeyCode::Up | KeyCode::Char('k') => app.table_prev(),
        _ => {}
    }
}

fn handle_gauge_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.memory_usage = (app.memory_usage + 0.05).min(1.0);
            app.disk_usage = (app.disk_usage + 0.05).min(1.0);
        }
        KeyCode::Char('-') => {
            app.memory_usage = (app.memory_usage - 0.05).max(0.0);
            app.disk_usage = (app.disk_usage - 0.05).max(0.0);
        }
        KeyCode::Char('r') => {
            app.download_progress = 0.0;
        }
        _ => {}
    }
}

// ============================================================
// Drawing
// ============================================================

fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Main layout: tabs at top, content in middle, status bar at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(size);

    // -- Tab bar --
    let titles: Vec<Line> = app
        .tab_titles
        .iter()
        .map(|t| Line::from(*t))
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ratatui showcase ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .select(app.current_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .divider(symbols::DOT);

    f.render_widget(tabs, chunks[0]);

    // -- Content --
    match app.current_tab {
        0 => draw_todo_tab(f, app, chunks[1]),
        1 => draw_table_tab(f, app, chunks[1]),
        2 => draw_gauge_tab(f, app, chunks[1]),
        3 => draw_chart_tab(f, app, chunks[1]),
        4 => draw_about_tab(f, chunks[1]),
        _ => {}
    }

    // -- Status bar --
    let elapsed = app.start_time.elapsed().as_secs();
    let status = Line::from(vec![
        Span::styled(" Tab/Shift+Tab", Style::default().fg(Color::Yellow).bold()),
        Span::raw(": switch  "),
        Span::styled("↑↓/jk", Style::default().fg(Color::Yellow).bold()),
        Span::raw(": navigate  "),
        Span::styled("?", Style::default().fg(Color::Yellow).bold()),
        Span::raw(": help  "),
        Span::styled("q", Style::default().fg(Color::Yellow).bold()),
        Span::raw(": quit  "),
        Span::styled(
            format!("  uptime: {}:{:02}", elapsed / 60, elapsed % 60),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[2]);

    // -- Help popup --
    if app.show_help {
        draw_help_popup(f, size);
    }

    // -- Todo input popup --
    if app.adding_todo {
        draw_input_popup(f, app, size);
    }
}

// ============================================================
// Tab 0: Todo List
// ============================================================

fn draw_todo_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: todo list
    let done_count = app.todos.iter().filter(|t| t.done).count();
    let total = app.todos.len();

    let items: Vec<ListItem> = app
        .todos
        .iter()
        .map(|todo| {
            let (icon, style) = if todo.done {
                ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::CROSSED_OUT))
            } else {
                ("○", Style::default().fg(Color::White))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default().fg(if todo.done { Color::Green } else { Color::DarkGray })),
                Span::styled(&todo.text, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Todo ({done_count}/{total} done) "))
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[0], &mut app.todo_state);

    // Right: instructions + progress
    let progress = if total > 0 { done_count as f64 / total as f64 } else { 0.0 };

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // progress gauge
            Constraint::Min(0),    // instructions
        ])
        .split(chunks[1]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Progress "))
        .gauge_style(
            Style::default()
                .fg(if progress >= 1.0 { Color::Green } else { Color::Cyan })
                .bg(Color::Black),
        )
        .percent((progress * 100.0) as u16)
        .label(format!("{:.0}%", progress * 100.0));

    f.render_widget(gauge, right_chunks[0]);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑/↓ or j/k", Style::default().fg(Color::Yellow).bold()),
            Span::raw("  Navigate"),
        ]),
        Line::from(vec![
            Span::styled("  Enter/Space", Style::default().fg(Color::Yellow).bold()),
            Span::raw("  Toggle done"),
        ]),
        Line::from(vec![
            Span::styled("  a", Style::default().fg(Color::Yellow).bold()),
            Span::raw("          Add new todo"),
        ]),
        Line::from(vec![
            Span::styled("  d/Del", Style::default().fg(Color::Yellow).bold()),
            Span::raw("      Delete todo"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  This demonstrates:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  • StatefulWidget (ListState)",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  • Highlight styles",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  • Dynamic list modification",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  • Popup input overlay",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  • Gauge widget for progress",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let instructions = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Controls ")
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(instructions, right_chunks[1]);
}

// ============================================================
// Tab 1: Table
// ============================================================

fn draw_table_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Crate").style(Style::default().fg(Color::Yellow).bold()),
        Cell::from("Version").style(Style::default().fg(Color::Yellow).bold()),
        Cell::from("Description").style(Style::default().fg(Color::Yellow).bold()),
        Cell::from("Rating").style(Style::default().fg(Color::Yellow).bold()),
    ])
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .table_data
        .iter()
        .map(|row| {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .map(|(i, text)| {
                    let style = match i {
                        0 => Style::default().fg(Color::Cyan).bold(),
                        1 => Style::default().fg(Color::Green),
                        3 => Style::default().fg(Color::Yellow),
                        _ => Style::default(),
                    };
                    Cell::from(text.as_str()).style(style)
                })
                .collect();
            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Min(20),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Popular Rust Crates ")
            .title_style(Style::default().fg(Color::Cyan).bold()),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

// ============================================================
// Tab 2: Gauges
// ============================================================

fn draw_gauge_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" System Gauges ")
        .title_style(Style::default().fg(Color::Cyan).bold());
    f.render_widget(block, area);

    // CPU — animated, colored by severity
    let cpu_color = if app.cpu_usage > 0.8 {
        Color::Red
    } else if app.cpu_usage > 0.5 {
        Color::Yellow
    } else {
        Color::Green
    };

    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(cpu_color).bg(Color::Black))
        .percent((app.cpu_usage * 100.0) as u16)
        .label(format!("{:.1}%", app.cpu_usage * 100.0));
    f.render_widget(cpu_gauge, chunks[0]);

    // Memory
    let mem_color = if app.memory_usage > 0.8 { Color::Red } else { Color::Magenta };
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(mem_color).bg(Color::Black))
        .percent((app.memory_usage * 100.0) as u16)
        .label(format!("{:.1}% ({:.1} / 16.0 GB)", app.memory_usage * 100.0, app.memory_usage * 16.0));
    f.render_widget(mem_gauge, chunks[1]);

    // Disk
    let disk_gauge = Gauge::default()
        .block(Block::default().title(" Disk ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
        .percent((app.disk_usage * 100.0) as u16)
        .label(format!("{:.0} / 512 GB", app.disk_usage * 512.0));
    f.render_widget(disk_gauge, chunks[2]);

    // Download — LineGauge variant
    let dl_gauge = LineGauge::default()
        .block(Block::default().title(" Download ").borders(Borders::ALL))
        .filled_style(Style::default().fg(Color::Cyan))
        .unfilled_style(Style::default().fg(Color::DarkGray))
        .line_set(symbols::line::THICK)
        .ratio(app.download_progress)
        .label(if app.download_progress >= 1.0 {
            "Complete ✓".to_string()
        } else {
            format!("{:.0}%", app.download_progress * 100.0)
        });
    f.render_widget(dl_gauge, chunks[3]);

    // Sparkline — CPU history
    let sparkline = Sparkline::default()
        .block(Block::default().title(" CPU History (sparkline) ").borders(Borders::ALL))
        .data(&app.sparkline_data)
        .max(100)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline, chunks[4]);

    // Instructions
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" +/- ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("adjust memory/disk  "),
        Span::styled(" r ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("restart download  "),
        Span::styled("  CPU and download animate automatically", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(help, chunks[5]);
}

// ============================================================
// Tab 3: Charts
// ============================================================

fn draw_chart_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Line chart
    let datasets = vec![
        Dataset::default()
            .name("sin(x) + cos(2x)")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&app.chart_data),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Live Waveform (Chart widget) ")
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .x_axis(
            Axis::default()
                .title("x")
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, 10.0])
                .labels(vec![Line::from("0"), Line::from("5"), Line::from("10")]),
        )
        .y_axis(
            Axis::default()
                .title("y")
                .style(Style::default().fg(Color::DarkGray))
                .bounds([-4.0, 4.0])
                .labels(vec![Line::from("-4"), Line::from("0"), Line::from("4")]),
        );

    f.render_widget(chart, chunks[0]);

    // Bottom: sparkline with different styling
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let sparkline1 = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" CPU (green) "),
        )
        .data(&app.sparkline_data)
        .max(100)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline1, bottom_chunks[0]);

    // Generate inverted data for visual variety
    let inverted: Vec<u64> = app.sparkline_data.iter().map(|v| 100 - v).collect();
    let sparkline2 = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Idle (yellow) "),
        )
        .data(&inverted)
        .max(100)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(sparkline2, bottom_chunks[1]);
}

// ============================================================
// Tab 4: About
// ============================================================

fn draw_about_tab(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🦀 ratatui-showcase",
            Style::default().fg(Color::Cyan).bold().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(""),
        Line::from("  A comprehensive demo of ratatui's capabilities."),
        Line::from(""),
        Line::from(Span::styled("  Widgets demonstrated:", Style::default().fg(Color::Yellow).bold())),
        Line::from(""),
        Line::from("    ✓ Tabs          — Tab navigation with highlight styles"),
        Line::from("    ✓ List          — Stateful, navigable, with custom symbols"),
        Line::from("    ✓ Table         — Headers, styled cells, row selection"),
        Line::from("    ✓ Gauge         — Standard + LineGauge, colored by threshold"),
        Line::from("    ✓ Sparkline     — Live-updating data visualization"),
        Line::from("    ✓ Chart         — Animated line chart with braille markers"),
        Line::from("    ✓ Paragraph     — Styled text with wrapping"),
        Line::from("    ✓ Block         — Borders, titles, styling"),
        Line::from("    ✓ Popup/Overlay — Centered overlay for input + help"),
        Line::from(""),
        Line::from(Span::styled("  Patterns demonstrated:", Style::default().fg(Color::Yellow).bold())),
        Line::from(""),
        Line::from("    ✓ Event loop with tick-based animation"),
        Line::from("    ✓ Stateful widgets (ListState, TableState)"),
        Line::from("    ✓ Layout with Constraints (horizontal + vertical splits)"),
        Line::from("    ✓ Popup overlays with Clear widget"),
        Line::from("    ✓ Text input handling in raw mode"),
        Line::from("    ✓ Terminal setup/restore (raw mode, alternate screen)"),
        Line::from("    ✓ Global vs tab-specific keybindings"),
        Line::from(""),
        Line::from(Span::styled(
            "  Built with ratatui + crossterm",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  github.com/turtleeverywhere/ratatui-showcase",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" About ")
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

// ============================================================
// Popups
// ============================================================

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 60, area);
    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("  Global", Style::default().fg(Color::Yellow).bold())),
        Line::from("    Tab / Shift+Tab   Switch tabs"),
        Line::from("    ?                 Toggle this help"),
        Line::from("    q / Esc           Quit"),
        Line::from(""),
        Line::from(Span::styled("  Todo Tab", Style::default().fg(Color::Yellow).bold())),
        Line::from("    ↑/↓ or j/k        Navigate"),
        Line::from("    Enter / Space     Toggle done"),
        Line::from("    a                 Add new todo"),
        Line::from("    d / Delete        Delete todo"),
        Line::from(""),
        Line::from(Span::styled("  Table Tab", Style::default().fg(Color::Yellow).bold())),
        Line::from("    ↑/↓ or j/k        Navigate rows"),
        Line::from(""),
        Line::from(Span::styled("  Gauges Tab", Style::default().fg(Color::Yellow).bold())),
        Line::from("    +/-               Adjust memory/disk"),
        Line::from("    r                 Restart download"),
        Line::from(""),
        Line::from(Span::styled("  Press Esc to close", Style::default().fg(Color::DarkGray))),
    ];

    let popup = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ❓ Help ")
                .title_style(Style::default().fg(Color::Cyan).bold())
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(popup, popup_area);
}

fn draw_input_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ✏️  Add Todo ")
        .title_style(Style::default().fg(Color::Cyan).bold())
        .style(Style::default().bg(Color::Black));
    f.render_widget(block, popup_area);

    let label = Paragraph::new(Span::styled(
        "  Type your todo and press Enter (Esc to cancel):",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(label, input_chunks[0]);

    let input = Paragraph::new(format!("  {}_", app.todo_input))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(" Input "));
    f.render_widget(input, input_chunks[1]);
}
