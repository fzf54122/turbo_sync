use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};

use anyhow::{Context, Result};
use serde::Serialize;
use tauri::{CustomMenuItem, Manager, Menu, MenuItem, State, Submenu};
use turbosync_core::{
    config,
    models::{
        CreateNodeRequest, CreateTaskRequest, FileListResponse, LogsResponse, Node, StatusResponse,
        SyncTask, UpdateTaskRequest, WatchStatus,
    },
};

#[derive(Default)]
struct AgentProcess {
    child: Mutex<Option<Child>>,
}

#[derive(Clone, Serialize)]
struct AgentReadyResponse {
    started: bool,
    base_url: String,
}

#[derive(Clone, Serialize)]
struct Snapshot {
    status: StatusResponse,
    nodes: Vec<Node>,
    tasks: Vec<SyncTask>,
    logs: LogsResponse,
    watched_task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentCommandSpec {
    program: PathBuf,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
}

#[derive(Clone)]
struct AgentClient {
    base_url: String,
    client: reqwest::Client,
}

pub fn run() {
    tauri::Builder::default()
        .menu(app_menu())
        .on_menu_event(|event| match event.menu_item_id() {
            "refresh" => {
                let _ = event.window().emit("menu://refresh", ());
            }
            "toggle_theme" => {
                let _ = event.window().emit("menu://toggle-theme", ());
            }
            "about" => {
                let _ = event.window().emit("menu://about", ());
            }
            "minimize" => {
                let _ = event.window().minimize();
            }
            "toggle_maximize" => {
                if event.window().is_maximized().unwrap_or(false) {
                    let _ = event.window().unmaximize();
                } else {
                    let _ = event.window().maximize();
                }
            }
            "close" => {
                let _ = event.window().close();
            }
            _ => {}
        })
        .manage(AgentProcess::default())
        .invoke_handler(tauri::generate_handler![
            ensure_agent_ready,
            load_snapshot,
            add_node,
            remove_node,
            add_task,
            update_task,
            remove_task,
            sync_task,
            rescan_task,
            start_watch,
            stop_watch,
            list_task_files,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build TurboSync GUI")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                stop_managed_agent(app);
            }
        });
}

fn app_menu() -> Menu {
    let app_menu = Menu::new()
        .add_item(CustomMenuItem::new("refresh", "刷新状态").accelerator("CmdOrCtrl+R"))
        .add_item(CustomMenuItem::new("toggle_theme", "切换主题").accelerator("CmdOrCtrl+T"))
        .add_native_item(MenuItem::Separator)
        .add_item(CustomMenuItem::new("about", "关于飞梭同步"));

    let sync_menu = Menu::new()
        .add_item(CustomMenuItem::new("sync_selected", "同步选中任务"))
        .add_item(CustomMenuItem::new("rescan_selected", "重新扫描"))
        .add_item(CustomMenuItem::new("watch_selected", "开启监听"));

    let node_menu = Menu::new()
        .add_item(CustomMenuItem::new("add_node", "添加节点"))
        .add_item(CustomMenuItem::new("health_check", "检查节点健康"))
        .add_item(CustomMenuItem::new("cert_help", "证书指纹说明"));

    let window_menu = Menu::new()
        .add_item(CustomMenuItem::new("minimize", "最小化").accelerator("CmdOrCtrl+M"))
        .add_item(CustomMenuItem::new("toggle_maximize", "最大化/还原"))
        .add_native_item(MenuItem::Separator)
        .add_item(CustomMenuItem::new("close", "关闭窗口").accelerator("CmdOrCtrl+Q"));

    let help_menu = Menu::new()
        .add_item(CustomMenuItem::new("quick_start", "快速开始"))
        .add_item(CustomMenuItem::new("self_test", "自测流程"))
        .add_item(CustomMenuItem::new("issue", "反馈问题"));

    Menu::new()
        .add_submenu(Submenu::new("应用", app_menu))
        .add_submenu(Submenu::new("同步", sync_menu))
        .add_submenu(Submenu::new("节点", node_menu))
        .add_submenu(Submenu::new("窗口", window_menu))
        .add_submenu(Submenu::new("帮助", help_menu))
}

#[tauri::command]
async fn ensure_agent_ready(state: State<'_, AgentProcess>) -> Result<AgentReadyResponse, String> {
    ensure_local_agent_running(&state).await.map_err(to_error)
}

#[tauri::command]
async fn load_snapshot() -> Result<Snapshot, String> {
    load_snapshot_inner().await.map_err(to_error)
}

#[tauri::command]
async fn add_node(request: CreateNodeRequest) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client.post_json::<_, Node>("/v1/nodes", &request).await?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn remove_node(node_id: String) -> Result<Snapshot, String> {
    run_agent_action(|client| async move { client.delete(&format!("/v1/nodes/{node_id}")).await })
        .await
}

#[tauri::command]
async fn add_task(request: CreateTaskRequest) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client
            .post_json::<_, SyncTask>("/v1/tasks", &request)
            .await?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn update_task(task_id: String, updates: UpdateTaskRequest) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client
            .put_json::<_, SyncTask>(&format!("/v1/tasks/{task_id}"), &updates)
            .await?;
        Ok(())
    })
    .await
}

#[tauri::command]
async fn remove_task(task_id: String) -> Result<Snapshot, String> {
    run_agent_action(|client| async move { client.delete(&format!("/v1/tasks/{task_id}")).await })
        .await
}

#[tauri::command]
async fn sync_task(task_id: String) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client
            .post_empty(&format!("/v1/tasks/{task_id}/sync"))
            .await
    })
    .await
}

#[tauri::command]
async fn rescan_task(task_id: String) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client
            .post_empty(&format!("/v1/tasks/{task_id}/rescan"))
            .await
    })
    .await
}

#[tauri::command]
async fn start_watch(task_id: String) -> Result<Snapshot, String> {
    run_agent_action(|client| async move {
        client
            .post_empty(&format!("/v1/tasks/{task_id}/watch"))
            .await
    })
    .await
}

#[tauri::command]
async fn stop_watch(task_id: String) -> Result<Snapshot, String> {
    run_agent_action(
        |client| async move { client.delete(&format!("/v1/tasks/{task_id}/watch")).await },
    )
    .await
}

#[tauri::command]
async fn list_task_files(task_id: String) -> Result<FileListResponse, String> {
    let client = AgentClient::from_config().map_err(to_error)?;
    client
        .get_json(&format!("/v1/tasks/{task_id}/files"))
        .await
        .map_err(to_error)
}

async fn run_agent_action<F, Fut>(action: F) -> Result<Snapshot, String>
where
    F: FnOnce(AgentClient) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let client = AgentClient::from_config().map_err(to_error)?;
    action(client.clone()).await.map_err(to_error)?;
    load_snapshot_with_client(&client).await.map_err(to_error)
}

async fn ensure_local_agent_running(state: &AgentProcess) -> Result<AgentReadyResponse> {
    if let Ok(client) = AgentClient::from_config() {
        if client.health_check().await.is_ok() {
            return Ok(AgentReadyResponse {
                started: false,
                base_url: client.base_url,
            });
        }
    }

    let paths = config::resolve_paths()?;
    if !paths.config_file.exists() {
        config::init_config_at(&paths)?;
    }

    let specs = agent_command_specs()?;
    let mut last_error = None;

    for spec in specs {
        match spawn_agent(&spec) {
            Ok(mut child) => {
                for _ in 0..50 {
                    if let Ok(client) = AgentClient::from_config() {
                        if client.health_check().await.is_ok() {
                            replace_managed_agent(state, child);
                            return Ok(AgentReadyResponse {
                                started: true,
                                base_url: client.base_url,
                            });
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(120)).await;
                }

                terminate_child(&mut child);
                last_error = Some(format!("{} 启动后未就绪", command_label(&spec)));
            }
            Err(error) => {
                last_error = Some(format!("{} 启动失败：{error}", command_label(&spec)));
            }
        }
    }

    anyhow::bail!(
        "{}",
        last_error.unwrap_or_else(|| "没有找到可用的本机同步服务启动命令".to_owned())
    );
}

fn replace_managed_agent(state: &AgentProcess, child: Child) {
    if let Ok(mut guard) = state.child.lock() {
        if let Some(mut old_child) = guard.take() {
            terminate_child(&mut old_child);
        }
        *guard = Some(child);
    }
}

fn stop_managed_agent(app: &tauri::AppHandle) {
    if let Ok(mut guard) = app.state::<AgentProcess>().child.lock() {
        if let Some(mut child) = guard.take() {
            terminate_child(&mut child);
        }
    }
}

fn spawn_agent(spec: &AgentCommandSpec) -> Result<Child> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Put cargo/agent into a dedicated process group so closing the GUI
        // also stops descendants spawned by `cargo run` during development.
        // SAFETY: pre_exec only calls async-signal-safe setsid before exec and
        // returns the OS error immediately if the process group cannot be created.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    if let Some(current_dir) = &spec.current_dir {
        command.current_dir(current_dir);
    }

    command.spawn().with_context(|| command_label(spec))
}

fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    // SAFETY: child.id() is the process-group leader because spawn_agent calls
    // setsid before exec; sending SIGTERM to the negative pid targets only that
    // dedicated process group.
    unsafe {
        let process_group_id = -(child.id() as i32);
        let _ = libc::kill(process_group_id, libc::SIGTERM);
    }

    let _ = child.kill();
    let _ = child.wait();
}

fn agent_command_specs() -> Result<Vec<AgentCommandSpec>> {
    let current_exe = std::env::current_exe().context("无法定位当前 GUI 可执行文件")?;
    Ok(agent_command_specs_for(&current_exe))
}

fn agent_command_specs_for(current_exe: &Path) -> Vec<AgentCommandSpec> {
    let mut specs = Vec::new();

    if let Some(exe_dir) = current_exe.parent() {
        push_sibling_agent_specs(&mut specs, exe_dir);
    }
    if let Some(repo_root) = find_repo_root(current_exe) {
        push_repo_agent_specs(&mut specs, &repo_root);
        push_cargo_agent_specs(&mut specs, &repo_root);
    }
    push_path_agent_specs(&mut specs);

    specs
}

fn push_sibling_agent_specs(specs: &mut Vec<AgentCommandSpec>, exe_dir: &Path) {
    specs.push(AgentCommandSpec {
        program: exe_dir.join(exe_name("turbosync-agent")),
        args: Vec::new(),
        current_dir: None,
    });
    specs.push(AgentCommandSpec {
        program: exe_dir.join(exe_name("tsync")),
        args: vec!["agent".into(), "run".into()],
        current_dir: None,
    });
}

fn push_repo_agent_specs(specs: &mut Vec<AgentCommandSpec>, repo_root: &Path) {
    for profile in ["debug", "release"] {
        let bin_dir = repo_root.join("target").join(profile);
        specs.push(AgentCommandSpec {
            program: bin_dir.join(exe_name("turbosync-agent")),
            args: Vec::new(),
            current_dir: None,
        });
        specs.push(AgentCommandSpec {
            program: bin_dir.join(exe_name("tsync")),
            args: vec!["agent".into(), "run".into()],
            current_dir: None,
        });
    }
}

fn push_cargo_agent_specs(specs: &mut Vec<AgentCommandSpec>, repo_root: &Path) {
    specs.push(AgentCommandSpec {
        program: PathBuf::from("cargo"),
        args: vec![
            "run".into(),
            "-p".into(),
            "turbosync-agent".into(),
            "--".into(),
        ],
        current_dir: Some(repo_root.to_path_buf()),
    });
    specs.push(AgentCommandSpec {
        program: PathBuf::from("cargo"),
        args: vec![
            "run".into(),
            "-p".into(),
            "turbosync-cli".into(),
            "--".into(),
            "agent".into(),
            "run".into(),
        ],
        current_dir: Some(repo_root.to_path_buf()),
    });
}

fn push_path_agent_specs(specs: &mut Vec<AgentCommandSpec>) {
    specs.push(AgentCommandSpec {
        program: PathBuf::from(exe_name("turbosync-agent")),
        args: Vec::new(),
        current_dir: None,
    });
    specs.push(AgentCommandSpec {
        program: PathBuf::from(exe_name("tsync")),
        args: vec!["agent".into(), "run".into()],
        current_dir: None,
    });
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join("crates/turbosync-cli").is_dir() && ancestor.join("Cargo.toml").is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn command_label(spec: &AgentCommandSpec) -> String {
    let mut parts = vec![spec.program.display().to_string()];
    parts.extend(spec.args.iter().cloned());
    parts.join(" ")
}

async fn load_snapshot_inner() -> Result<Snapshot> {
    let client = AgentClient::from_config()?;
    load_snapshot_with_client(&client).await
}

async fn load_snapshot_with_client(client: &AgentClient) -> Result<Snapshot> {
    let status = client.get_json("/v1/status").await?;
    let nodes = client.get_json("/v1/nodes").await?;
    let tasks: Vec<SyncTask> = client.get_json("/v1/tasks").await?;
    let logs = client.get_json("/v1/logs?limit=50").await?;
    let mut watched_task_ids = Vec::new();

    for task in &tasks {
        if let Ok(status) = client
            .get_json::<WatchStatus>(&format!("/v1/tasks/{}/watch", task.id))
            .await
        {
            watched_task_ids.push(status.task_id);
        }
    }

    Ok(Snapshot {
        status,
        nodes,
        tasks,
        logs,
        watched_task_ids,
    })
}

fn to_error(error: anyhow::Error) -> String {
    error.to_string()
}

impl AgentClient {
    fn from_config() -> Result<Self> {
        let paths = config::resolve_paths()?;
        let config = config::load_config_at(&paths)
            .with_context(|| "TurboSync 未初始化，请先运行 tsync init")?;
        Ok(Self {
            base_url: format!("http://{}", config.agent_addr),
            client: reqwest::Client::new(),
        })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .get(self.url(path))
            .send()
            .await
            .context("agent 未启动，请先运行 tsync agent run")?;
        self.expect_json(response).await
    }

    async fn health_check(&self) -> Result<()> {
        let response = self
            .client
            .get(self.url("/health"))
            .send()
            .await
            .context("本机同步服务未启动")?;
        self.expect_status(response).await
    }

    async fn post_json<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<U> {
        let response = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .context("agent 未启动，请先运行 tsync agent run")?;
        self.expect_json(response).await
    }

    async fn put_json<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<U> {
        let response = self
            .client
            .put(self.url(path))
            .json(body)
            .send()
            .await
            .context("agent 未启动，请先运行 tsync agent run")?;
        self.expect_json(response).await
    }

    async fn post_empty(&self, path: &str) -> Result<()> {
        let response = self
            .client
            .post(self.url(path))
            .send()
            .await
            .context("agent 未启动，请先运行 tsync agent run")?;
        self.expect_status(response).await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let response = self
            .client
            .delete(self.url(path))
            .send()
            .await
            .context("agent 未启动，请先运行 tsync agent run")?;
        self.expect_status(response).await
    }

    async fn expect_json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let response = self.expect_success(response).await?;
        response.json().await.context("无法解析 agent 响应")
    }

    async fn expect_status(&self, response: reqwest::Response) -> Result<()> {
        self.expect_success(response).await.map(|_| ())
    }

    async fn expect_success(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "agent 返回未知错误".to_owned());
        anyhow::bail!("agent 请求失败 ({status}): {message}")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_adds_windows_suffix_only_on_windows() {
        let name = exe_name("tsync");
        if cfg!(windows) {
            assert_eq!(name, "tsync.exe");
        } else {
            assert_eq!(name, "tsync");
        }
    }

    #[test]
    fn command_label_includes_arguments() {
        let spec = AgentCommandSpec {
            program: PathBuf::from("tsync"),
            args: vec!["agent".to_owned(), "run".to_owned()],
            current_dir: None,
        };

        assert_eq!(command_label(&spec), "tsync agent run");
    }

    #[test]
    fn agent_command_specs_include_sibling_and_path_fallbacks() {
        let current_exe = PathBuf::from("/tmp/turbosync/bin/tsync-gui");
        let specs = agent_command_specs_for(&current_exe);

        assert!(specs
            .iter()
            .any(|spec| spec.program.ends_with(exe_name("turbosync-agent"))));
        assert!(
            specs
                .iter()
                .any(|spec| spec.program.ends_with(exe_name("tsync"))
                    && spec.args == ["agent", "run"])
        );
    }

    #[test]
    fn agent_command_specs_include_cargo_fallback_in_repo() {
        let current_exe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/tsync-gui");
        let specs = agent_command_specs_for(&current_exe);

        assert!(specs.iter().any(|spec| spec.program.as_os_str() == "cargo"
            && spec.args == ["run", "-p", "turbosync-agent", "--"]));
    }
}
