use std::{collections::HashSet, io, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use reqwest::StatusCode;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use turbosync_core::display::{format_bytes, format_unix_time};
use turbosync_core::{config, models::*};

// ── Entry point ─────────────────────────────────────────────────────────

pub async fn run() -> Result<()> {
    let client = DashboardClient::from_config()?;
    let mut terminal = TerminalSession::enter()?;
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiEvent>();
    let mut app = App::new(client, ui_tx.clone());

    app.refresh_all().await;

    let sse_client = app.client.clone();
    tokio::spawn(async move {
        sse_listen_loop(sse_client, ui_tx).await;
    });

    let mut fallback_ticks: u8 = 0;

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.help_open {
                    match key.code {
                        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                            app.help_open = false;
                        }
                        KeyCode::Char('l') => app.toggle_language(),
                        _ => {}
                    }
                    continue;
                }
                if app.edit_modal.is_some() {
                    app.handle_edit_key(key.code).await;
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.view == View::Files {
                            app.view = View::Tasks;
                            continue;
                        }
                        break;
                    }
                    KeyCode::Char('j') | KeyCode::Down => app.next_item(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous_item(),
                    KeyCode::Char('?') => app.help_open = true,
                    KeyCode::Char('l') => app.toggle_language(),
                    KeyCode::Char('r') => app.refresh_all().await,
                    KeyCode::Char('s') => app.sync_selected(),
                    KeyCode::Char('w') => app.start_watch_selected().await,
                    KeyCode::Char('x') => app.stop_watch_selected().await,
                    KeyCode::Char('e') => app.open_edit_modal(),
                    KeyCode::Char('f') => app.toggle_file_view().await,
                    _ => {}
                }
            }
        }

        while let Ok(event) = ui_rx.try_recv() {
            fallback_ticks = 60; // reset fallback timer
            app.handle_ui_event(event).await;
        }

        // Fallback polling — refresh every ~6s if SSE is silent
        fallback_ticks = fallback_ticks.saturating_sub(1);
        if fallback_ticks == 0 {
            app.refresh_all().await;
            fallback_ticks = 60;
        }
    }

    Ok(())
}

enum UiEvent {
    Sse(SseEvent),
    SyncFinished {
        task_name: String,
        result: std::result::Result<SyncResponse, String>,
    },
}

async fn sse_listen_loop(client: DashboardClient, tx: mpsc::UnboundedSender<UiEvent>) {
    loop {
        match client.connect_sse().await {
            Ok(response) => {
                let mut pending = String::new();
                let mut stream = response.bytes_stream();
                while let Some(Ok(chunk)) = StreamExt::next(&mut stream).await {
                    pending.push_str(&String::from_utf8_lossy(&chunk));
                    for event in drain_sse_events(&mut pending) {
                        let _ = tx.send(UiEvent::Sse(event));
                    }
                }
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

fn drain_sse_events(pending: &mut String) -> Vec<SseEvent> {
    let mut events = Vec::new();
    while let Some(pos) = pending.find("\n\n") {
        let frame: String = pending.drain(..pos + 2).collect();
        if let Some(event) = parse_sse_frame(&frame) {
            events.push(event);
        }
    }
    events
}

fn parse_sse_frame(frame: &str) -> Option<SseEvent> {
    let data = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() {
        return None;
    }
    serde_json::from_str(&data).ok()
}

// ── Terminal session ────────────────────────────────────────────────────

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

// ── View mode ───────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum View {
    Tasks,
    Files,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Zh,
    En,
}

impl Language {
    fn toggle(self) -> Self {
        match self {
            Self::Zh => Self::En,
            Self::En => Self::Zh,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "English",
        }
    }

    fn pick(self, zh: &'static str, en: &'static str) -> &'static str {
        match self {
            Self::Zh => zh,
            Self::En => en,
        }
    }
}

// ── Edit modal state ────────────────────────────────────────────────────

struct EditModal {
    task_id: String,
    name: String,
    source_path: String,
    target_path: String,
    enabled: bool,
    field: EditField,
}

#[derive(Clone, Copy)]
enum EditField {
    Name,
    SourcePath,
    TargetPath,
    Enabled,
}

// ── App ─────────────────────────────────────────────────────────────────

struct App {
    client: DashboardClient,
    ui_tx: mpsc::UnboundedSender<UiEvent>,
    status: Option<StatusResponse>,
    nodes: Vec<Node>,
    tasks: Vec<SyncTask>,
    logs: Vec<SyncRun>,
    watched_tasks: HashSet<String>,
    selected_task: usize,
    selected_file: usize,
    message: String,
    language: Language,
    view: View,
    file_list: Vec<FileIndexEntry>,
    edit_modal: Option<EditModal>,
    help_open: bool,
    progress: Option<SyncProgressEvent>,
}

impl App {
    fn new(client: DashboardClient, ui_tx: mpsc::UnboundedSender<UiEvent>) -> Self {
        Self {
            client,
            ui_tx,
            status: None,
            nodes: Vec::new(),
            tasks: Vec::new(),
            logs: Vec::new(),
            watched_tasks: HashSet::new(),
            selected_task: 0,
            selected_file: 0,
            message: "j/k 选择 · s 同步 · e 编辑 · f 文件 · w 监听 · l English · ? 帮助 · q 退出"
                .to_owned(),
            language: Language::Zh,
            view: View::Tasks,
            file_list: Vec::new(),
            edit_modal: None,
            help_open: false,
            progress: None,
        }
    }

    async fn refresh_all(&mut self) {
        match self.client.status().await {
            Ok(status) => self.status = Some(status),
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("Agent 不可用：{error}"),
                    Language::En => format!("agent unavailable: {error}"),
                };
                self.status = None;
            }
        }

        match self.client.list_nodes().await {
            Ok(nodes) => self.nodes = nodes,
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("加载节点失败：{error}"),
                    Language::En => format!("failed to load nodes: {error}"),
                };
            }
        }

        match self.client.list_tasks().await {
            Ok(tasks) => {
                self.tasks = tasks;
                if self.selected_task >= self.tasks.len() {
                    self.selected_task = self.tasks.len().saturating_sub(1);
                }
                self.refresh_watch_status().await;
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("加载任务失败：{error}"),
                    Language::En => format!("failed to load tasks: {error}"),
                };
            }
        }

        match self.client.logs(50).await {
            Ok(response) => self.logs = response.runs,
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("加载日志失败：{error}"),
                    Language::En => format!("failed to load logs: {error}"),
                };
            }
        }

        if self.view == View::Files {
            self.refresh_file_list().await;
        }
    }

    async fn refresh_watch_status(&mut self) {
        let mut watched = HashSet::new();
        for task in &self.tasks {
            if self.client.watch_status(&task.id).await.is_ok() {
                watched.insert(task.id.clone());
            }
        }
        self.watched_tasks = watched;
    }

    async fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::Sse(event) => match event.event.as_str() {
                "sync_progress" => {
                    if let Ok(progress) = serde_json::from_value(event.data) {
                        self.progress = Some(progress);
                    }
                }
                "sync_run_completed" => {
                    self.progress = None;
                    self.refresh_all().await;
                }
                "task_updated" | "watch_triggered" => {
                    self.refresh_all().await;
                }
                _ => {}
            },
            UiEvent::SyncFinished { task_name, result } => {
                self.progress = None;
                match result {
                    Ok(response) => {
                        self.message = match self.language {
                            Language::Zh => format!(
                                "同步 {}：{} 个操作，{} 个失败",
                                task_name,
                                response.operations.len(),
                                response.run.files_failed
                            ),
                            Language::En => format!(
                                "sync {}: {} operations, {} failed",
                                task_name,
                                response.operations.len(),
                                response.run.files_failed
                            ),
                        };
                        self.refresh_all().await;
                    }
                    Err(error) => {
                        self.message = match self.language {
                            Language::Zh => format!("同步失败：{error}"),
                            Language::En => format!("sync failed: {error}"),
                        };
                        self.refresh_all().await;
                    }
                }
            }
        }
    }

    fn toggle_language(&mut self) {
        self.language = self.language.toggle();
        self.message = match self.language {
            Language::Zh => "已切换为中文界面".to_owned(),
            Language::En => "Switched to English UI".to_owned(),
        };
    }

    fn next_item(&mut self) {
        match self.view {
            View::Tasks => {
                if !self.tasks.is_empty() {
                    self.selected_task = (self.selected_task + 1).min(self.tasks.len() - 1);
                }
            }
            View::Files => {
                if !self.file_list.is_empty() {
                    self.selected_file = (self.selected_file + 1).min(self.file_list.len() - 1);
                }
            }
        }
    }

    fn previous_item(&mut self) {
        match self.view {
            View::Tasks => {
                self.selected_task = self.selected_task.saturating_sub(1);
            }
            View::Files => {
                self.selected_file = self.selected_file.saturating_sub(1);
            }
        }
    }

    // ── sync / watch ────────────────────────────────────────────────

    fn sync_selected(&mut self) {
        if self.progress.is_some() {
            self.message = match self.language {
                Language::Zh => "同步已经在进行中，请等完成后再按 s".to_owned(),
                Language::En => "sync is already running; wait before pressing s again".to_owned(),
            };
            return;
        }

        let Some(task) = self.selected_task().cloned() else {
            self.message = match self.language {
                Language::Zh => "未选择任务".to_owned(),
                Language::En => "no task selected".to_owned(),
            };
            return;
        };

        self.progress = Some(SyncProgressEvent {
            task_id: task.id.clone(),
            current: 0,
            total: 0,
            message: match self.language {
                Language::Zh => "开始同步".to_owned(),
                Language::En => "starting sync".to_owned(),
            },
        });
        self.message = match self.language {
            Language::Zh => format!("正在同步 {}", task.name),
            Language::En => format!("syncing {}", task.name),
        };

        let client = self.client.clone();
        let tx = self.ui_tx.clone();
        tokio::spawn(async move {
            let result = client
                .sync_task(&task.id)
                .await
                .map_err(|error| error.to_string());
            let _ = tx.send(UiEvent::SyncFinished {
                task_name: task.name,
                result,
            });
        });
    }

    async fn start_watch_selected(&mut self) {
        let Some(task) = self.selected_task().cloned() else {
            self.message = match self.language {
                Language::Zh => "未选择任务".to_owned(),
                Language::En => "no task selected".to_owned(),
            };
            return;
        };

        match self.client.start_watch(&task.id).await {
            Ok(_) => {
                self.watched_tasks.insert(task.id.clone());
                self.message = match self.language {
                    Language::Zh => format!("已开启监听 {}", task.name),
                    Language::En => format!("watching {}", task.name),
                };
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("开启监听失败：{error}"),
                    Language::En => format!("watch failed: {error}"),
                };
            }
        }
    }

    async fn stop_watch_selected(&mut self) {
        let Some(task) = self.selected_task().cloned() else {
            self.message = match self.language {
                Language::Zh => "未选择任务".to_owned(),
                Language::En => "no task selected".to_owned(),
            };
            return;
        };

        match self.client.stop_watch(&task.id).await {
            Ok(_) => {
                self.watched_tasks.remove(&task.id);
                self.message = match self.language {
                    Language::Zh => format!("已停止监听 {}", task.name),
                    Language::En => format!("stopped watching {}", task.name),
                };
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("停止监听失败：{error}"),
                    Language::En => format!("stop watch failed: {error}"),
                };
            }
        }
    }

    // ── edit modal ──────────────────────────────────────────────────

    fn open_edit_modal(&mut self) {
        let Some(task) = self.selected_task() else {
            self.message = match self.language {
                Language::Zh => "未选择任务".to_owned(),
                Language::En => "no task selected".to_owned(),
            };
            return;
        };
        self.edit_modal = Some(EditModal {
            task_id: task.id.clone(),
            name: task.name.clone(),
            source_path: task.source_path.clone(),
            target_path: task.target_path.clone(),
            enabled: task.enabled,
            field: EditField::Name,
        });
    }

    async fn handle_edit_key(&mut self, key: KeyCode) {
        if self.edit_modal.is_none() {
            return;
        }

        match key {
            KeyCode::Esc => self.edit_modal = None,
            KeyCode::Enter => self.save_edit_modal().await,
            KeyCode::Tab => {
                let modal = self.edit_modal.as_mut().unwrap();
                modal.field = next_edit_field(modal.field);
            }
            KeyCode::Backspace => self.edit_backspace(),
            KeyCode::Char(' ') => self.toggle_edit_enabled(),
            KeyCode::Char(c) => self.edit_push_char(c),
            _ => {}
        }
    }

    async fn save_edit_modal(&mut self) {
        let Some(modal) = self.edit_modal.take() else {
            return;
        };

        match self
            .client
            .update_task(
                &modal.task_id,
                &modal.name,
                &modal.source_path,
                &modal.target_path,
                modal.enabled,
            )
            .await
        {
            Ok(_) => {
                self.message = match self.language {
                    Language::Zh => "任务已更新".to_owned(),
                    Language::En => "task updated".to_owned(),
                };
                self.refresh_all().await;
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("更新失败：{error}"),
                    Language::En => format!("update failed: {error}"),
                };
                self.edit_modal = Some(modal);
            }
        }
    }

    fn edit_backspace(&mut self) {
        let Some(modal) = self.edit_modal.as_mut() else {
            return;
        };
        match modal.field {
            EditField::Name => {
                modal.name.pop();
            }
            EditField::SourcePath => {
                modal.source_path.pop();
            }
            EditField::TargetPath => {
                modal.target_path.pop();
            }
            EditField::Enabled => {}
        }
    }

    fn edit_push_char(&mut self, c: char) {
        let Some(modal) = self.edit_modal.as_mut() else {
            return;
        };
        match modal.field {
            EditField::Name => modal.name.push(c),
            EditField::SourcePath => modal.source_path.push(c),
            EditField::TargetPath => modal.target_path.push(c),
            EditField::Enabled => {}
        }
    }

    fn toggle_edit_enabled(&mut self) {
        let Some(modal) = self.edit_modal.as_mut() else {
            return;
        };
        if matches!(modal.field, EditField::Enabled) {
            modal.enabled = !modal.enabled;
        }
    }

    // ── file browser ────────────────────────────────────────────────

    async fn toggle_file_view(&mut self) {
        if self.view == View::Files {
            self.view = View::Tasks;
            return;
        }

        let Some(task_id) = self.selected_task().map(|task| task.id.clone()) else {
            self.message = match self.language {
                Language::Zh => "未选择任务".to_owned(),
                Language::En => "no task selected".to_owned(),
            };
            return;
        };

        self.selected_file = 0;
        match self.client.list_files(&task_id).await {
            Ok(files) => {
                self.file_list = files;
                self.view = View::Files;
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("加载文件失败：{error}"),
                    Language::En => format!("failed to load files: {error}"),
                };
            }
        }
    }

    fn selected_task(&self) -> Option<&SyncTask> {
        self.tasks.get(self.selected_task)
    }

    fn selected_file(&self) -> Option<&FileIndexEntry> {
        self.file_list.get(self.selected_file)
    }

    async fn refresh_file_list(&mut self) {
        let Some(task_id) = self.selected_task().map(|task| task.id.clone()) else {
            self.file_list.clear();
            self.selected_file = 0;
            return;
        };

        match self.client.list_files(&task_id).await {
            Ok(files) => {
                self.file_list = files;
                if self.selected_file >= self.file_list.len() {
                    self.selected_file = self.file_list.len().saturating_sub(1);
                }
            }
            Err(error) => {
                self.message = match self.language {
                    Language::Zh => format!("加载文件失败：{error}"),
                    Language::En => format!("failed to load files: {error}"),
                };
            }
        }
    }
}

fn next_edit_field(field: EditField) -> EditField {
    match field {
        EditField::Name => EditField::SourcePath,
        EditField::SourcePath => EditField::TargetPath,
        EditField::TargetPath => EditField::Enabled,
        EditField::Enabled => EditField::Name,
    }
}

fn status_label(language: Language, status: &str) -> String {
    match (language, status) {
        (Language::Zh, "running") => "运行中".to_owned(),
        (Language::Zh, "success") => "成功".to_owned(),
        (Language::Zh, "failed") => "失败".to_owned(),
        (Language::Zh, "ok") => "正常".to_owned(),
        (_, "success") => "OK".to_owned(),
        (_, "failed") => "FAIL".to_owned(),
        (_, other) => other.to_uppercase(),
    }
}

fn direction_label(language: Language, direction: &str) -> String {
    match (language, direction) {
        (Language::Zh, "one_way") => "单向同步".to_owned(),
        (Language::Zh, "two_way") => "双向同步".to_owned(),
        (_, "one_way") => "one-way".to_owned(),
        (_, "two_way") => "two-way".to_owned(),
        (_, other) => other.to_owned(),
    }
}

fn trigger_label(language: Language, trigger: &str) -> String {
    match (language, trigger) {
        (Language::Zh, "manual_sync") => "手动同步".to_owned(),
        (Language::Zh, "manual_rescan") => "手动扫描".to_owned(),
        (Language::Zh, "watcher") => "文件变化".to_owned(),
        (_, "manual_sync") => "manual sync".to_owned(),
        (_, "manual_rescan") => "manual rescan".to_owned(),
        (_, "watcher") => "watcher".to_owned(),
        (_, other) => other.to_owned(),
    }
}

fn enabled_label(language: Language, enabled: bool) -> &'static str {
    match (language, enabled) {
        (Language::Zh, true) => "启用",
        (Language::Zh, false) => "停用",
        (_, true) => "enabled",
        (_, false) => "disabled",
    }
}

fn watch_label(language: Language, watching: bool) -> &'static str {
    match (language, watching) {
        (Language::Zh, true) => "监听中",
        (Language::Zh, false) => "未监听",
        (_, true) => "watching",
        (_, false) => "stopped",
    }
}

fn node_health_label(language: Language, status: &str) -> &'static str {
    match (language, status) {
        (Language::Zh, "connected") => "已连接",
        (Language::Zh, "failed") => "连接失败",
        (Language::Zh, _) => "未检测",
        (_, "connected") => "connected",
        (_, "failed") => "failed",
        (_, _) => "unchecked",
    }
}

fn node_health_color(status: &str) -> Color {
    match status {
        "connected" => Color::Green,
        "failed" => Color::Red,
        _ => Color::Yellow,
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────

fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let has_progress = app.progress.is_some();
    let bottom_height = if has_progress { 4 } else { 3 };

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(bottom_height),
        ])
        .split(area);

    draw_header(frame, root[0], app);
    draw_body(frame, root[1], app);
    draw_footer(frame, root[2], app);

    if app.edit_modal.is_some() {
        draw_edit_modal(frame, app);
    }
    if app.help_open {
        draw_help_modal(frame, app);
    }
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let connected_nodes = app
        .nodes
        .iter()
        .filter(|node| node.health_status == "connected")
        .count();
    let failed_nodes = app
        .nodes
        .iter()
        .filter(|node| node.health_status == "failed")
        .count();
    let status = match &app.status {
        Some(status) => vec![
            Span::styled("● ", Style::default().fg(Color::Green)),
            Span::styled(
                lang.pick("Agent 在线", "Agent online"),
                Style::default().fg(Color::Green),
            ),
            Span::raw(lang.pick("  节点 ", "  node ")),
            Span::styled(short_id(&status.node_id), Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{}{}  {}{}  {}{}  {}{}  {}{}  {}{}",
                lang.pick("  已连节点 ", "  nodes "),
                status.enabled_nodes,
                lang.pick("  可连接 ", "  connected "),
                connected_nodes,
                lang.pick("  异常 ", "  failed "),
                failed_nodes,
                lang.pick("  同步任务 ", "  tasks "),
                status.enabled_tasks,
                lang.pick("  待处理 ", "  pending "),
                status.pending_events,
                lang.pick("  最近同步 ", "  last "),
                format_unix_time(status.last_sync_at.as_deref(), "never")
            )),
        ],
        None => vec![
            Span::styled("● ", Style::default().fg(Color::Red)),
            Span::styled(
                lang.pick("Agent 离线", "Agent offline"),
                Style::default().fg(Color::Red),
            ),
            Span::raw(lang.pick(
                "  请先运行：tsync agent run",
                "  start with: tsync agent run",
            )),
        ],
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "TurboSync",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::raw(lang.pick("文件同步控制台", "file sync console")),
            Span::raw("  "),
            Span::styled(
                format!("[{}]", lang.code()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(status),
    ])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn draw_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    match app.view {
        View::Tasks => {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
                .split(area);
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(12), Constraint::Min(8)])
                .split(columns[1]);

            draw_tasks(frame, columns[0], app);
            draw_task_detail(frame, right[0], app);
            draw_logs(frame, right[1], app);
        }
        View::Files => {
            draw_file_browser(frame, area, app);
        }
    }
}

fn draw_task_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let lines = match app.selected_task() {
        Some(task) => {
            let watching = app.watched_tasks.contains(&task.id);
            let last_run = app
                .logs
                .iter()
                .find(|run| run.task_id.as_deref() == Some(task.id.as_str()));
            let target_node = app.nodes.iter().find(|node| node.id == task.target_node_id);
            let target_node_name = target_node
                .map(|node| format!("{} {}", node.name, short_id(&node.id)))
                .unwrap_or_else(|| short_id(&task.target_node_id));
            let target_endpoint = target_node
                .map(|node| node.endpoint.as_str())
                .unwrap_or("-");
            let target_health = target_node
                .map(|node| node.health_status.as_str())
                .unwrap_or("unchecked");
            let target_health_message = target_node.and_then(|node| node.health_message.as_deref());
            vec![
                Line::from(vec![
                    Span::styled(&task.name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(short_id(&task.id), Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("源目录  ", "Source  "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(&task.source_path),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("同步到  ", "Target  "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!("{target_node_name}:{}", task.target_path)),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("连接    ", "Link    "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        node_health_label(lang, target_health),
                        Style::default().fg(node_health_color(target_health)),
                    ),
                    Span::raw(format!(" · {target_endpoint}")),
                    Span::styled(
                        target_health_message
                            .map(|message| format!(" · {message}"))
                            .unwrap_or_default(),
                        Style::default().fg(Color::Red),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("策略    ", "Policy  "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!(
                        "{} · {} · {}",
                        direction_label(lang, &task.direction),
                        task.delete_mode,
                        task.conflict_mode
                    )),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("状态    ", "State   "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(format!(
                        "{} · {}",
                        enabled_label(lang, task.enabled),
                        watch_label(lang, watching)
                    )),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("操作    ", "Actions "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(if app.progress.is_some() {
                        lang.pick(
                            "同步进行中，s 已锁定  w 监听  x 停止  f 文件",
                            "sync running, s locked  w watch  x stop  f files",
                        )
                    } else {
                        lang.pick(
                            "s 同步  w 监听  x 停止  e 编辑  f 文件",
                            "s sync  w watch  x stop  e edit  f files",
                        )
                    }),
                ]),
                Line::from(vec![
                    Span::styled(
                        lang.pick("最近    ", "Latest  "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(match last_run {
                        Some(run) => format!(
                            "{} · {}={} · {}={} · {}={}",
                            status_label(lang, &run.status),
                            lang.pick("变更", "changed"),
                            run.files_changed,
                            lang.pick("失败", "failed"),
                            run.files_failed,
                            lang.pick("数据", "bytes"),
                            format_size(run.bytes_sent)
                        ),
                        None => lang.pick("还没有同步记录", "no sync runs yet").to_owned(),
                    }),
                ]),
            ]
        }
        None => vec![Line::from(lang.pick(
            "还没有同步任务。先用命令添加任务，再回到这里操作。",
            "No sync tasks yet. Add a task, then use this console.",
        ))],
    };

    let detail = Paragraph::new(lines)
        .block(
            Block::default()
                .title(lang.pick(" 当前同步链路 ", " Sync Route "))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(detail, area);
}

fn draw_tasks(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let items: Vec<ListItem<'_>> = if app.tasks.is_empty() {
        vec![ListItem::new(lang.pick(
            "暂无任务。使用 tsync task add 添加。",
            "No tasks. Use tsync task add ...",
        ))]
    } else {
        app.tasks
            .iter()
            .map(|task| {
                let watching = app.watched_tasks.contains(&task.id);
                let watch = if watching {
                    Span::styled("●", Style::default().fg(Color::Green))
                } else {
                    Span::styled("○", Style::default().fg(Color::DarkGray))
                };
                ListItem::new(vec![
                    Line::from(vec![
                        watch,
                        Span::raw(" "),
                        Span::styled(&task.name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("  "),
                        Span::styled(short_id(&task.id), Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![Span::styled(
                        format!(
                            "  {} · {}",
                            direction_label(lang, &task.direction),
                            watch_label(lang, watching)
                        ),
                        Style::default().fg(Color::DarkGray),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  {} -> {}", task.source_path, task.target_path),
                        Style::default().fg(Color::DarkGray),
                    )]),
                ])
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(lang.pick(" 同步任务 ", " Sync Tasks "))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol("› ");
    let mut state =
        ListState::default().with_selected((!app.tasks.is_empty()).then_some(app.selected_task));

    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_logs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let items: Vec<ListItem<'_>> = if app.logs.is_empty() {
        vec![ListItem::new(lang.pick(
            "暂无同步活动。按 s 开始一次同步。",
            "No activity yet. Press s to sync.",
        ))]
    } else {
        app.logs
            .iter()
            .take(8)
            .map(|run| {
                let color = match run.status.as_str() {
                    "success" => Color::Green,
                    "failed" => Color::Red,
                    _ => Color::Yellow,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(status_label(lang, &run.status), Style::default().fg(color)),
                    Span::raw("  "),
                    Span::styled(
                        trigger_label(lang, &run.trigger_kind),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(format!(
                        "  {}={}  {}={}  {}={}  {}",
                        lang.pick("变更", "changed"),
                        run.files_changed,
                        lang.pick("失败", "failed"),
                        run.files_failed,
                        lang.pick("数据", "bytes"),
                        format_size(run.bytes_sent),
                        format_unix_time(
                            run.finished_at.as_deref(),
                            lang.pick("进行中", "running")
                        )
                    )),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .title(lang.pick(" 最近活动 ", " Recent Activity "))
            .borders(Borders::ALL),
    );
    frame.render_widget(list, area);
}

fn draw_file_browser(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let panes = if area.width >= 100 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(9)])
            .split(area)
    };

    let items: Vec<ListItem<'_>> = if app.file_list.is_empty() {
        vec![ListItem::new(lang.pick(
            "还没有文件索引。按 s 同步或运行 rescan。",
            "No files indexed. Press s to sync or run rescan.",
        ))]
    } else {
        app.file_list
            .iter()
            .map(|entry| {
                let hash = entry
                    .content_hash
                    .as_deref()
                    .map(short_id)
                    .unwrap_or_else(|| "-".to_owned());
                let size = entry
                    .size_bytes
                    .map(format_size)
                    .unwrap_or_else(|| "-".to_owned());
                let deleted = if entry.deleted {
                    Span::styled(" D ", Style::default().fg(Color::Red))
                } else {
                    Span::styled("   ", Style::default())
                };
                let synced = if entry.last_synced_at.is_some() {
                    Span::styled("✓", Style::default().fg(Color::Green))
                } else {
                    Span::styled(" ", Style::default())
                };

                let kind_color = if entry.file_kind == "dir" {
                    Color::Cyan
                } else {
                    Color::White
                };
                let state = if entry.deleted {
                    lang.pick("已删除", "deleted")
                } else if entry.last_synced_at.is_some() {
                    lang.pick("已同步", "synced")
                } else {
                    lang.pick("待同步", "pending")
                };

                ListItem::new(Line::from(vec![
                    deleted,
                    synced,
                    Span::raw(" "),
                    Span::styled(&entry.relative_path, Style::default().fg(kind_color)),
                    Span::raw("  "),
                    Span::styled(state, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(hash, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(size, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect()
    };

    let title = app
        .selected_task()
        .map(|t| format!(" {}: {} ", lang.pick("文件索引", "Files"), t.name))
        .unwrap_or_else(|| format!(" {} ", lang.pick("文件索引", "Files")));

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol("› ");
    let mut state = ListState::default()
        .with_selected((!app.file_list.is_empty()).then_some(app.selected_file));

    frame.render_stateful_widget(list, panes[0], &mut state);
    draw_file_detail(frame, panes[1], app);
}

fn draw_file_detail(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let lines = match app.selected_file() {
        Some(entry) => vec![
            Line::from(vec![
                Span::styled(
                    lang.pick("路径 ", "Path "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&entry.relative_path),
            ]),
            Line::from(vec![
                Span::styled(
                    lang.pick("类型 ", "Kind "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&entry.file_kind),
            ]),
            Line::from(vec![
                Span::styled(
                    lang.pick("大小 ", "Size "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(
                    entry
                        .size_bytes
                        .map(format_size)
                        .unwrap_or_else(|| "-".to_owned()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Hash ", Style::default().fg(Color::Yellow)),
                Span::raw(entry.content_hash.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    lang.pick("发现 ", "Seen "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&entry.last_seen_at),
            ]),
            Line::from(vec![
                Span::styled(
                    lang.pick("同步 ", "Synced "),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(
                    entry
                        .last_synced_at
                        .as_deref()
                        .unwrap_or_else(|| lang.pick("待同步", "pending")),
                ),
            ]),
        ],
        None => vec![Line::from(lang.pick("未选择文件。", "No file selected."))],
    };

    let detail = Paragraph::new(lines)
        .block(
            Block::default()
                .title(lang.pick(" 文件详情 ", " File Detail "))
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(detail, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lang = app.language;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.progress.is_some() {
            vec![Constraint::Length(1), Constraint::Length(3)]
        } else {
            vec![Constraint::Length(3)]
        })
        .split(area);

    let footer_area = if let Some(progress) = &app.progress {
        let ratio = if progress.total > 0 {
            progress.current as f64 / progress.total as f64
        } else {
            0.0
        };
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(Color::Cyan))
            .label(format!(
                " {}: {}/{} {}",
                lang.pick("同步进度", "Sync progress"),
                progress.current,
                progress.total,
                progress.message
            ))
            .ratio(ratio);
        frame.render_widget(gauge, chunks[0]);
        chunks[1]
    } else {
        chunks[0]
    };

    let footer = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(lang.pick("操作 ", "Keys "), Style::default().fg(Color::Yellow)),
            Span::raw(match lang {
                Language::Zh => {
                    "j/k 选择 · s 同步 · w/x 监听 · e 编辑 · f 文件 · r 刷新 · l English · ? 帮助 · q 退出"
                }
                Language::En => {
                    "j/k select · s sync · w/x watch · e edit · f files · r refresh · l 中文 · ? help · q quit"
                }
            }),
        ]),
        Line::from(vec![
            Span::styled(lang.pick("状态 ", "Status "), Style::default().fg(Color::Yellow)),
            Span::raw(&app.message),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL))
    .wrap(Wrap { trim: true });

    frame.render_widget(footer, footer_area);
}

fn draw_edit_modal(frame: &mut Frame<'_>, app: &App) {
    let Some(modal) = &app.edit_modal else {
        return;
    };
    let lang = app.language;

    let area = centered_rect(72, 11, frame.area());

    let name_style = match modal.field {
        EditField::Name => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default(),
    };
    let source_style = match modal.field {
        EditField::SourcePath => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default(),
    };
    let target_style = match modal.field {
        EditField::TargetPath => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default(),
    };
    let enabled_style = match modal.field {
        EditField::Enabled => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default(),
    };

    let content = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            lang.pick("编辑同步任务", "Edit Task"),
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![
            Span::styled(
                lang.pick("名称：", "Name: "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&modal.name, name_style),
        ]),
        Line::from(vec![
            Span::styled(
                lang.pick("源目录：", "Source: "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&modal.source_path, source_style),
        ]),
        Line::from(vec![
            Span::styled(
                lang.pick("目标目录：", "Target: "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(&modal.target_path, target_style),
        ]),
        Line::from(vec![
            Span::styled(
                lang.pick("启用：", "Enabled: "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                if modal.enabled {
                    lang.pick("是", "yes")
                } else {
                    lang.pick("否", "no")
                },
                enabled_style,
            ),
        ]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled(
            lang.pick(
                "Enter 保存  Tab 切换字段  Space 切换启用  Esc 取消",
                "Enter save  Tab switch field  Space toggle enabled  Esc cancel",
            ),
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(content, area);
}

fn draw_help_modal(frame: &mut Frame<'_>, app: &App) {
    let lang = app.language;
    let area = centered_rect(68, 13, frame.area());
    let lines = match lang {
        Language::Zh => vec![
            Line::from(vec![Span::styled(
                "TurboSync 同步控制台",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("j / k        移动选择"),
            Line::from("s            同步选中的任务，界面不会卡住"),
            Line::from("w / x        开启或停止选中任务的文件监听"),
            Line::from("e            编辑选中的任务"),
            Line::from("f            查看文件索引，再按一次返回"),
            Line::from("r            立即刷新"),
            Line::from("l            Switch to English"),
            Line::from("?            显示或关闭帮助"),
            Line::from("Esc / q      返回或退出"),
        ],
        Language::En => vec![
            Line::from(vec![Span::styled(
                "TurboSync Sync Console",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("j / k        Move selection"),
            Line::from("s            Sync selected task without blocking the UI"),
            Line::from("w / x        Start or stop watching the selected task"),
            Line::from("e            Edit selected task"),
            Line::from("f            Toggle indexed file browser"),
            Line::from("r            Refresh immediately"),
            Line::from("l            切换中文"),
            Line::from("?            Show or hide this help"),
            Line::from("Esc / q      Back or quit"),
        ],
    };
    let content = Paragraph::new(lines).block(
        Block::default()
            .title(lang.pick(" 帮助 ", " Help "))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(content, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
            Constraint::Length(percent_x),
            Constraint::Length((r.width.saturating_sub(percent_x)) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn format_size(bytes: i64) -> String {
    format_bytes(bytes)
}

// ── HTTP client ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct DashboardClient {
    base_url: String,
    client: reqwest::Client,
}

impl DashboardClient {
    fn from_config() -> Result<Self> {
        let paths = config::resolve_paths()?;
        let config = config::load_config_at(&paths)
            .with_context(|| "TurboSync is not initialized; run `tsync init` first")?;

        Ok(Self {
            base_url: format!("http://{}", config.agent_addr),
            client: reqwest::Client::new(),
        })
    }

    async fn status(&self) -> Result<StatusResponse> {
        self.get("/v1/status").await
    }

    async fn list_nodes(&self) -> Result<Vec<Node>> {
        self.get("/v1/nodes").await
    }

    async fn list_tasks(&self) -> Result<Vec<SyncTask>> {
        self.get("/v1/tasks").await
    }

    async fn logs(&self, limit: u16) -> Result<LogsResponse> {
        self.get(&format!("/v1/logs?limit={limit}")).await
    }

    async fn sync_task(&self, task_id: &str) -> Result<SyncResponse> {
        self.post_empty(&format!("/v1/tasks/{task_id}/sync")).await
    }

    async fn start_watch(&self, task_id: &str) -> Result<WatchStatus> {
        self.post_empty(&format!("/v1/tasks/{task_id}/watch")).await
    }

    async fn stop_watch(&self, task_id: &str) -> Result<bool> {
        self.delete(&format!("/v1/tasks/{task_id}/watch")).await
    }

    async fn watch_status(&self, task_id: &str) -> Result<WatchStatus> {
        self.get(&format!("/v1/tasks/{task_id}/watch")).await
    }

    async fn list_files(&self, task_id: &str) -> Result<Vec<FileIndexEntry>> {
        let response: FileListResponse = self.get(&format!("/v1/tasks/{task_id}/files")).await?;
        Ok(response.files)
    }

    async fn update_task(
        &self,
        task_id: &str,
        name: &str,
        source_path: &str,
        target_path: &str,
        enabled: bool,
    ) -> Result<SyncTask> {
        let response = self
            .client
            .put(self.url(&format!("/v1/tasks/{task_id}")))
            .json(&serde_json::json!({
                "name": name,
                "source_path": source_path,
                "target_path": target_path,
                "enabled": enabled,
            }))
            .send()
            .await
            .context("agent is not running")?
            .error_for_status()
            .context("agent request failed")?;
        response.json().await.context("failed to decode response")
    }

    async fn connect_sse(&self) -> Result<reqwest::Response> {
        self.client
            .get(self.url("/v1/events"))
            .send()
            .await
            .context("failed to connect to SSE endpoint")
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .get(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?
            .error_for_status()
            .context("agent request failed")?;

        response.json().await.context("failed to decode response")
    }

    async fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .post(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?
            .error_for_status()
            .context("agent request failed")?;

        response.json().await.context("failed to decode response")
    }

    async fn delete(&self, path: &str) -> Result<bool> {
        let response = self
            .client
            .delete(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            _ => {
                response
                    .error_for_status()
                    .context("agent request failed")?;
                Ok(true)
            }
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_id_truncates_ids() {
        assert_eq!(short_id("1234567890"), "12345678");
    }

    #[test]
    fn status_label_highlights_known_statuses() {
        assert_eq!(status_label(Language::Zh, "success"), "成功");
        assert_eq!(status_label(Language::Zh, "failed"), "失败");
        assert_eq!(status_label(Language::En, "success"), "OK");
        assert_eq!(status_label(Language::En, "running"), "RUNNING");
    }

    #[test]
    fn format_size_bytes_kb_mb() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(2_500_000), "2.4 MB");
    }

    #[test]
    fn parse_sse_frame_reads_json_data() {
        let event = parse_sse_frame(
            "event: sync_progress\ndata: {\"event\":\"sync_progress\",\"data\":{\"task_id\":\"t1\",\"current\":1,\"total\":2,\"message\":\"copying\"}}\n\n",
        )
        .unwrap();

        assert_eq!(event.event, "sync_progress");
        let progress: SyncProgressEvent = serde_json::from_value(event.data).unwrap();
        assert_eq!(progress.current, 1);
        assert_eq!(progress.total, 2);
    }
}
