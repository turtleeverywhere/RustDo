/// # ratatui-showcase
///
/// An interactive TUI demonstrating ratatui's widgets, layouts, and patterns.
/// Navigate between tabs to explore different widget types.
///
/// Controls:
///   Tab / Shift+Tab  — switch tabs
///   ↑/↓ or j/k       — navigate lists/tables
///   ←/→ or h/l        — switch pane (in Todo tab)
///   Enter             — toggle/select items
///   a                 — add project or todo (in Todo tab)
///   d/Delete          — delete item (in Todo tab)
///   +/-               — adjust gauge values
///   q / Esc           — quit
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
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

    /// Projects & Todos (Tab 0)
    projects: Vec<Project>,
    project_state: ListState,
    todo_state: ListState,
    todo_focus: TodoFocus,
    adding_todo: bool,
    todo_input: String,
    adding_project: bool,
    project_input: String,

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

    /// Delete confirmation
    confirm_delete: Option<DeleteTarget>,
}

#[derive(Clone)]
enum DeleteTarget {
    Project(usize),
    Todo { project_idx: usize, todo_idx: usize },
}

#[derive(PartialEq)]
enum TodoFocus {
    Projects,
    Todos,
}

struct Project {
    name: String,
    todos: Vec<TodoItem>,
}

struct TodoItem {
    text: String,
    done: bool,
    created_at: i64,
}

impl App {
    fn new(loaded_projects: Vec<Project>) -> Self {
        let projects = if loaded_projects.is_empty() {
            default_projects()
        } else {
            loaded_projects
        };

        let mut project_state = ListState::default();
        project_state.select(if projects.is_empty() { None } else { Some(0) });

        let mut todo_state = ListState::default();
        let has_todos = projects.first().map_or(false, |p| !p.todos.is_empty());
        todo_state.select(if has_todos { Some(0) } else { None });

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        App {
            current_tab: 0,
            tab_titles: vec!["📝 Todo", "📊 Table", "🔋 Gauges", "📈 Charts", "📖 About"],

            projects,
            project_state,
            todo_state,
            todo_focus: TodoFocus::Projects,
            adding_todo: false,
            todo_input: String::new(),
            adding_project: false,
            project_input: String::new(),

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
            confirm_delete: None,
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

    // -- Project actions --

    fn project_next(&mut self) {
        let len = self.projects.len();
        if len == 0 { return; }
        let i = self.project_state.selected().unwrap_or(0);
        self.project_state.select(Some((i + 1) % len));
        self.reset_todo_selection();
    }

    fn project_prev(&mut self) {
        let len = self.projects.len();
        if len == 0 { return; }
        let i = self.project_state.selected().unwrap_or(0);
        self.project_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
        self.reset_todo_selection();
    }

    fn project_add(&mut self) {
        if !self.project_input.is_empty() {
            self.projects.push(Project {
                name: self.project_input.drain(..).collect(),
                todos: Vec::new(),
            });
            self.project_state.select(Some(self.projects.len() - 1));
            self.todo_state.select(None);
        }
        self.adding_project = false;
    }

    fn project_delete(&mut self) {
        if let Some(i) = self.project_state.selected() {
            if i < self.projects.len() {
                self.projects.remove(i);
                if !self.projects.is_empty() {
                    let new_idx = if i >= self.projects.len() { self.projects.len() - 1 } else { i };
                    self.project_state.select(Some(new_idx));
                    self.reset_todo_selection();
                } else {
                    self.project_state.select(None);
                    self.todo_state.select(None);
                }
            }
        }
    }

    fn reset_todo_selection(&mut self) {
        if let Some(project) = self.project_state.selected().and_then(|i| self.projects.get(i)) {
            if project.todos.is_empty() {
                self.todo_state.select(None);
            } else {
                self.todo_state.select(Some(0));
            }
        } else {
            self.todo_state.select(None);
        }
    }

    // -- Todo actions --

    fn todo_next(&mut self) {
        if let Some(project) = self.project_state.selected().and_then(|i| self.projects.get(i)) {
            let len = project.todos.len();
            if len == 0 { return; }
            let i = self.todo_state.selected().unwrap_or(0);
            self.todo_state.select(Some((i + 1) % len));
        }
    }

    fn todo_prev(&mut self) {
        if let Some(project) = self.project_state.selected().and_then(|i| self.projects.get(i)) {
            let len = project.todos.len();
            if len == 0 { return; }
            let i = self.todo_state.selected().unwrap_or(0);
            self.todo_state.select(Some(if i == 0 { len - 1 } else { i - 1 }));
        }
    }

    fn todo_toggle(&mut self) {
        if let Some(i) = self.todo_state.selected() {
            if let Some(project) = self.project_state.selected().and_then(|pi| self.projects.get_mut(pi)) {
                if i < project.todos.len() {
                    project.todos[i].done = !project.todos[i].done;
                }
            }
        }
    }

    fn todo_delete(&mut self) {
        if let Some(i) = self.todo_state.selected() {
            if let Some(project) = self.project_state.selected().and_then(|pi| self.projects.get_mut(pi)) {
                if i < project.todos.len() {
                    project.todos.remove(i);
                    if !project.todos.is_empty() {
                        let new_idx = if i >= project.todos.len() { project.todos.len() - 1 } else { i };
                        self.todo_state.select(Some(new_idx));
                    } else {
                        self.todo_state.select(None);
                    }
                }
            }
        }
    }

    fn todo_add(&mut self) {
        if !self.todo_input.is_empty() {
            if let Some(project) = self.project_state.selected().and_then(|pi| self.projects.get_mut(pi)) {
                project.todos.push(TodoItem {
                    text: self.todo_input.drain(..).collect(),
                    done: false,
                    created_at: now_unix_secs(),
                });
                self.todo_state.select(Some(project.todos.len() - 1));
            }
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
// Storage — SQLite-backed persistence
// ============================================================

fn db_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("ratatui-showcase");
    std::fs::create_dir_all(&path).expect("failed to create data directory");
    path.push("app.db");
    path
}

fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS projects (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            name     TEXT    NOT NULL,
            position INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS todos (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            text       TEXT    NOT NULL,
            done       INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT 0,
            position   INTEGER NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );",
    )?;

    migrate_todos_created_at(conn)
}

fn migrate_todos_created_at(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(todos)")?;
    let has_created_at = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .flatten()
        .any(|column_name| column_name == "created_at");

    if !has_created_at {
        conn.execute(
            "ALTER TABLE todos ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "UPDATE todos SET created_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE created_at = 0",
            [],
        )?;
    }

    Ok(())
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn todo_fire_badge(created_at: i64, now: i64, done: bool) -> &'static str {
    if done || created_at <= 0 || now <= created_at {
        return "";
    }

    let age_days = ((now - created_at) / 86_400) as i32;
    match age_days {
        0 => "",
        1 | 2 => " 🔥",
        3 | 4 => " 🔥🔥",
        _ => " 🔥🔥🔥",
    }
}

fn load_projects(conn: &Connection) -> rusqlite::Result<Vec<Project>> {
    let mut project_stmt = conn.prepare("SELECT id, name FROM projects ORDER BY position")?;
    let project_rows: Vec<(i64, String)> = project_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    let mut projects = Vec::new();
    for (id, name) in project_rows {
        let mut todo_stmt =
            conn.prepare("SELECT text, done, created_at FROM todos WHERE project_id = ?1 ORDER BY position")?;
        let todos: Vec<TodoItem> = todo_stmt
            .query_map(params![id], |row| {
                Ok(TodoItem {
                    text: row.get(0)?,
                    done: row.get::<_, bool>(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<_, _>>()?;

        projects.push(Project { name, todos });
    }

    Ok(projects)
}

fn save_projects(conn: &Connection, projects: &[Project]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM todos", [])?;
    tx.execute("DELETE FROM projects", [])?;

    for (pos, project) in projects.iter().enumerate() {
        tx.execute(
            "INSERT INTO projects (name, position) VALUES (?1, ?2)",
            params![project.name, pos],
        )?;
        let project_id = conn.last_insert_rowid();
        for (todo_pos, todo) in project.todos.iter().enumerate() {
            tx.execute(
                "INSERT INTO todos (project_id, text, done, created_at, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![project_id, todo.text, todo.done, todo.created_at, todo_pos],
            )?;
        }
    }

    tx.commit()
}

fn default_projects() -> Vec<Project> {
    let now = now_unix_secs();
    vec![
        Project {
            name: "Learn Ratatui".into(),
            todos: vec![
                TodoItem { text: "Learn ratatui basics".into(), done: true, created_at: now - 6 * 86_400 },
                TodoItem { text: "Build a layout with constraints".into(), done: true, created_at: now - 4 * 86_400 },
                TodoItem { text: "Add keyboard navigation".into(), done: false, created_at: now - 2 * 86_400 },
                TodoItem { text: "Style widgets with colors".into(), done: false, created_at: now - 5 * 86_400 },
            ],
        },
        Project {
            name: "Build App".into(),
            todos: vec![
                TodoItem { text: "Create a stateful list".into(), done: false, created_at: now - 1 * 86_400 },
                TodoItem { text: "Handle events in a loop".into(), done: false, created_at: now - 3 * 86_400 },
                TodoItem { text: "Deploy to production 🚀".into(), done: false, created_at: now - 7 * 86_400 },
            ],
        },
    ]
}

// ============================================================
// Main — setup terminal, run event loop, restore terminal
// ============================================================

fn main() -> io::Result<()> {
    let conn = Connection::open(db_path()).expect("failed to open database");
    init_db(&conn).expect("failed to initialize database");
    let projects = load_projects(&conn).unwrap_or_default();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &conn, projects);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match result {
        Ok(projects) => {
            if let Err(e) = save_projects(&conn, &projects) {
                eprintln!("Warning: failed to save data: {e}");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &Connection,
    projects: Vec<Project>,
) -> io::Result<Vec<Project>> {
    let mut app = App::new(projects);
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        // Draw
        terminal.draw(|f| draw(f, &mut app))?;

        // Handle events with timeout
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // Process only real key presses to avoid duplicate handling on platforms
                // that also emit Repeat/Release events (notably Windows).
                if key.kind != KeyEventKind::Press {
                    continue;
                }

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

                // If adding a project, handle text input
                if app.adding_project {
                    match key.code {
                        KeyCode::Enter => app.project_add(),
                        KeyCode::Esc => {
                            app.adding_project = false;
                            app.project_input.clear();
                        }
                        KeyCode::Backspace => { app.project_input.pop(); }
                        KeyCode::Char(c) => app.project_input.push(c),
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

                // Delete confirmation
                if app.confirm_delete.is_some() {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Enter => {
                            if let Some(target) = app.confirm_delete.take() {
                                match target {
                                    DeleteTarget::Project(_) => app.project_delete(),
                                    DeleteTarget::Todo { .. } => app.todo_delete(),
                                }
                            }
                        }
                        _ => app.confirm_delete = None,
                    }
                    continue;
                }

                // Global keys
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char('s') => {
                        if let Err(e) = save_projects(conn, &app.projects) {
                            eprintln!("Warning: failed to save data: {e}");
                        }
                    }
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
            return Ok(app.projects);
        }
    }
}

fn handle_todo_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Left | KeyCode::Char('h') => {
            app.todo_focus = TodoFocus::Projects;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if !app.projects.is_empty() {
                app.todo_focus = TodoFocus::Todos;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => match app.todo_focus {
            TodoFocus::Projects => app.project_next(),
            TodoFocus::Todos => app.todo_next(),
        },
        KeyCode::Up | KeyCode::Char('k') => match app.todo_focus {
            TodoFocus::Projects => app.project_prev(),
            TodoFocus::Todos => app.todo_prev(),
        },
        KeyCode::Enter | KeyCode::Char(' ') => match app.todo_focus {
            TodoFocus::Projects => {
                if !app.projects.is_empty() {
                    app.todo_focus = TodoFocus::Todos;
                }
            }
            TodoFocus::Todos => app.todo_toggle(),
        },
        KeyCode::Char('a') => match app.todo_focus {
            TodoFocus::Projects => {
                app.adding_project = true;
                app.project_input.clear();
            }
            TodoFocus::Todos => {
                if !app.projects.is_empty() {
                    app.adding_todo = true;
                    app.todo_input.clear();
                }
            }
        },
        KeyCode::Char('d') | KeyCode::Delete => match app.todo_focus {
            TodoFocus::Projects => {
                if let Some(idx) = app.project_state.selected() {
                    if idx < app.projects.len() {
                        app.confirm_delete = Some(DeleteTarget::Project(idx));
                    }
                }
            }
            TodoFocus::Todos => {
                if let (Some(pi), Some(ti)) =
                    (app.project_state.selected(), app.todo_state.selected())
                {
                    if pi < app.projects.len() && ti < app.projects[pi].todos.len() {
                        app.confirm_delete =
                            Some(DeleteTarget::Todo { project_idx: pi, todo_idx: ti });
                    }
                }
            }
        },
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

    // -- Input popups --
    if app.adding_todo {
        draw_input_popup(f, &app.todo_input, " ✏️  Add Todo ", "Type your todo and press Enter (Esc to cancel):", size);
    }
    if app.adding_project {
        draw_input_popup(f, &app.project_input, " 📁 Add Project ", "Type project name and press Enter (Esc to cancel):", size);
    }

    if let Some(target) = &app.confirm_delete {
        let message = match target {
            DeleteTarget::Project(idx) => {
                let name = app.projects.get(*idx).map_or("?", |p| &p.name);
                format!("Delete project \"{name}\" and all its todos?")
            }
            DeleteTarget::Todo { project_idx, todo_idx } => {
                let text = app.projects.get(*project_idx)
                    .and_then(|p| p.todos.get(*todo_idx))
                    .map_or("?", |t| &t.text);
                format!("Delete todo \"{text}\"?")
            }
        };
        draw_confirm_popup(f, &message, size);
    }
}

// ============================================================
// Tab 0: Todo List
// ============================================================

fn draw_todo_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let now = now_unix_secs();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(45),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Left: project list
    let project_items: Vec<ListItem> = app
        .projects
        .iter()
        .map(|project| {
            let done = project.todos.iter().filter(|t| t.done).count();
            let total = project.todos.len();
            ListItem::new(Line::from(vec![
                Span::styled(" 📁 ", Style::default().fg(Color::Yellow)),
                Span::raw(&project.name),
                Span::styled(
                    format!(" ({done}/{total})"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let project_border = if app.todo_focus == TodoFocus::Projects {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let project_list = List::new(project_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(project_border)
                .title(format!(" Projects ({}) ", app.projects.len()))
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(project_list, chunks[0], &mut app.project_state);

    // Middle: todo list for selected project
    let (todo_items, done_count, total, project_name) =
        if let Some(project) = app.project_state.selected().and_then(|i| app.projects.get(i)) {
            let done = project.todos.iter().filter(|t| t.done).count();
            let total = project.todos.len();
            let items: Vec<ListItem> = project
                .todos
                .iter()
                .map(|todo| {
                    let (icon, style) = if todo.done {
                        ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::CROSSED_OUT))
                    } else {
                        ("○", Style::default().fg(Color::White))
                    };
                    let fire_badge = todo_fire_badge(todo.created_at, now, todo.done);
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {icon} "),
                            Style::default().fg(if todo.done { Color::Green } else { Color::DarkGray }),
                        ),
                        Span::styled(format!("{}{}", todo.text, fire_badge), style),
                    ]))
                })
                .collect();
            (items, done, total, Some(project.name.clone()))
        } else {
            (vec![], 0, 0, None)
        };

    let todo_border = if app.todo_focus == TodoFocus::Todos {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let todo_title = match &project_name {
        Some(name) => format!(" {name} ({done_count}/{total} done) "),
        None => " Select a project ".to_string(),
    };

    let todo_list = List::new(todo_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(todo_border)
                .title(todo_title)
                .title_style(Style::default().fg(Color::Cyan).bold()),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(todo_list, chunks[1], &mut app.todo_state);

    // Right: progress + instructions
    let progress = if total > 0 { done_count as f64 / total as f64 } else { 0.0 };

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(chunks[2]);

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

    let focus_label = match app.todo_focus {
        TodoFocus::Projects => "Projects",
        TodoFocus::Todos => "Todos",
    };

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Focus: {focus_label}"),
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ←/→ h/l", Style::default().fg(Color::Yellow).bold()),
            Span::raw("  Switch pane"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓ j/k", Style::default().fg(Color::Yellow).bold()),
            Span::raw("  Navigate"),
        ]),
        Line::from(vec![
            Span::styled("  Enter", Style::default().fg(Color::Yellow).bold()),
            Span::raw("     Toggle / Enter"),
        ]),
        Line::from(vec![
            Span::styled("  a", Style::default().fg(Color::Yellow).bold()),
            Span::raw("         Add item"),
        ]),
        Line::from(vec![
            Span::styled("  d/Del", Style::default().fg(Color::Yellow).bold()),
            Span::raw("     Delete item"),
        ]),
        Line::from(vec![
            Span::styled("  s", Style::default().fg(Color::Yellow).bold()),
            Span::raw("         Save data"),
        ]),
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
        Line::from("    ✓ Persistent storage (embedded SQLite)"),
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
        Line::from("    s                 Save data"),
        Line::from("    q / Esc           Quit"),
        Line::from(""),
        Line::from(Span::styled("  Todo Tab", Style::default().fg(Color::Yellow).bold())),
        Line::from("    ←/→ or h/l        Switch pane"),
        Line::from("    ↑/↓ or j/k        Navigate"),
        Line::from("    Enter / Space     Enter project / Toggle"),
        Line::from("    a                 Add project or todo"),
        Line::from("    d / Delete        Delete item"),
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

fn draw_input_popup(f: &mut Frame, input_text: &str, title: &str, label_text: &str, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(3)])
        .split(popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .style(Style::default().bg(Color::Black));
    f.render_widget(block, popup_area);

    let label = Paragraph::new(Span::styled(
        format!("  {label_text}"),
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(label, input_chunks[0]);

    let input = Paragraph::new(format!("  {input_text}_"))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(" Input "));
    f.render_widget(input, input_chunks[1]);
}

fn draw_confirm_popup(f: &mut Frame, message: &str, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {message}"),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y / Enter", Style::default().fg(Color::Red).bold()),
            Span::raw("  confirm   "),
            Span::styled("any other key", Style::default().fg(Color::Green).bold()),
            Span::raw("  cancel"),
        ]),
    ];

    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ⚠ Confirm Delete ")
                .title_style(Style::default().fg(Color::Red).bold())
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(popup, popup_area);
}
