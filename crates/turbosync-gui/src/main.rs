use std::{rc::Rc, thread};

use anyhow::{Context, Result};
use serde::Deserialize;
use slint::{Model, ModelRc, VecModel};
use turbosync_core::{
    config,
    display::{format_bytes, format_unix_time},
    models::{CreateNodeRequest, CreateTaskRequest, LogsResponse, Node, StatusResponse, SyncTask},
};

slint::slint! {
    import { Button, LineEdit, ScrollView, VerticalBox, HorizontalBox } from "std-widgets.slint";

    export struct NodeItem {
        id: string,
        name: string,
        endpoint: string,
        health: string,
        detail: string,
    }

    export struct TaskItem {
        id: string,
        short_id: string,
        name: string,
        route: string,
        meta: string,
    }

    export struct LogItem {
        status: string,
        trigger: string,
        summary: string,
        time: string,
    }

    component Metric inherits Rectangle {
        in property <string> label;
        in property <string> value;
        border-color: #E5E5EA;
        border-width: 1px;
        border-radius: 14px;
        background: #FFFFFF;
        height: 62px;

        VerticalLayout {
            padding: 10px;
            spacing: 4px;
            Text { text: root.label; color: #6E6E73; font-size: 12px; }
            Text {
                text: root.value;
                color: #1D1D1F;
                font-size: 17px;
                font-weight: 700;
                overflow: elide;
            }
        }
    }

    component SectionTitle inherits Rectangle {
        in property <string> title;
        in property <string> subtitle;
        height: 52px;
        background: #FFFFFF;
        border-color: #E5E5EA;
        border-width: 1px;

        HorizontalLayout {
            padding-left: 14px;
            padding-right: 14px;
            Text { text: root.title; font-size: 15px; font-weight: 700; color: #1D1D1F; vertical-alignment: center; }
            Rectangle { horizontal-stretch: 1; }
            Text { text: root.subtitle; color: #6E6E73; font-size: 12px; vertical-alignment: center; overflow: elide; }
        }
    }

    component Panel inherits Rectangle {
        border-color: #E5E5EA;
        border-width: 1px;
        border-radius: 16px;
        background: #FFFFFF;
    }

    component MenuButton inherits Rectangle {
        in property <string> text;
        in property <bool> primary;
        in property <bool> enabled: true;
        callback clicked();
        height: 30px;
        border-radius: 15px;
        background: primary ? #007AFF : enabled ? #FFFFFF : #F2F2F7;
        border-color: primary ? #007AFF : #E5E5EA;
        border-width: 1px;

        TouchArea {
            enabled: root.enabled;
            clicked => { root.clicked(); }
        }

        Text {
            text: root.text;
            color: primary ? white : enabled ? #1D1D1F : #8E8E93;
            font-size: 12px;
            font-weight: primary ? 700 : 500;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    component SidebarItem inherits Rectangle {
        in property <string> title;
        in property <string> subtitle;
        in property <bool> active;
        callback clicked();
        height: 56px;
        border-radius: 12px;
        background: active ? #FFFFFF : transparent;

        TouchArea { clicked => { root.clicked(); } }

        VerticalLayout {
            padding-left: 12px;
            padding-right: 12px;
            spacing: 2px;
            Text { text: root.title; color: #1D1D1F; font-size: 13px; font-weight: 700; vertical-alignment: center; }
            Text { text: root.subtitle; color: #6E6E73; font-size: 11px; overflow: elide; }
        }
    }

    component NodeRow inherits Rectangle {
        in property <NodeItem> node;
        in property <bool> selected;
        callback choose(string);
        callback remove(string);
        height: 96px;
        background: selected ? #e8f4f2 : white;
        border-color: #d7ded9;
        border-width: 1px;

        TouchArea {
            clicked => { root.choose(root.node.id); }
        }

        VerticalLayout {
            padding: 12px;
            spacing: 7px;
            HorizontalLayout {
                Text {
                    text: root.node.name;
                    font-size: 14px;
                    font-weight: 700;
                    color: #18211f;
                    overflow: elide;
                    horizontal-stretch: 1;
                }
                Text {
                    text: root.node.health;
                    color: root.node.health == "已连接" ? #15803d : root.node.health == "连接失败" ? #b42318 : #b45309;
                    font-size: 12px;
                    vertical-alignment: center;
                }
            }
            Text { text: root.node.endpoint; color: #65716d; font-size: 12px; overflow: elide; }
            HorizontalLayout {
                Text { text: root.node.detail; color: #65716d; font-size: 12px; overflow: elide; horizontal-stretch: 1; }
                Button { text: "删除"; clicked => { root.remove(root.node.id); } }
            }
        }
    }

    component TaskRow inherits Rectangle {
        in property <TaskItem> task;
        callback sync(string);
        callback rescan(string);
        callback watch(string);
        callback stop_watch(string);
        callback remove(string);
        callback choose(string);
        in property <bool> selected;
        height: 116px;
        background: selected ? #F2F7FF : white;
        border-color: #E5E5EA;
        border-width: 1px;

        TouchArea { clicked => { root.choose(root.task.id); } }

        VerticalLayout {
            padding: 12px;
            spacing: 7px;
            Text {
                text: root.task.name + "  " + root.task.short_id;
                font-size: 14px;
                font-weight: 700;
                color: #18211f;
                overflow: elide;
            }
            Text { text: root.task.route; color: #65716d; font-size: 12px; overflow: elide; }
            Text { text: root.task.meta; color: #65716d; font-size: 12px; overflow: elide; }
            HorizontalLayout {
                spacing: 6px;
                Button { text: "同步"; clicked => { root.sync(root.task.id); } }
                Button { text: "扫描"; clicked => { root.rescan(root.task.id); } }
                Button { text: "监听"; clicked => { root.watch(root.task.id); } }
                Button { text: "停止"; clicked => { root.stop_watch(root.task.id); } }
                Button { text: "删除"; clicked => { root.remove(root.task.id); } }
            }
        }
    }

    component LogRow inherits Rectangle {
        in property <LogItem> log;
        height: 58px;
        background: white;
        border-color: #d7ded9;
        border-width: 1px;

        HorizontalLayout {
            padding-left: 12px;
            padding-right: 12px;
            spacing: 12px;
            Text {
                text: root.log.status;
                color: root.log.status == "成功" ? #15803d : root.log.status == "失败" ? #b42318 : #b45309;
                font-size: 13px;
                font-weight: 700;
                width: 58px;
                vertical-alignment: center;
            }
            Text { text: root.log.trigger; color: blue; font-size: 13px; width: 86px; vertical-alignment: center; overflow: elide; }
            Text { text: root.log.summary; color: #18211f; font-size: 13px; horizontal-stretch: 1; vertical-alignment: center; overflow: elide; }
            Text { text: root.log.time; color: #65716d; font-size: 12px; width: 130px; vertical-alignment: center; overflow: elide; }
        }
    }

    export component MainWindow inherits Window {
        title: "TurboSync";
        width: 1180px;
        height: 760px;
        background: #F5F5F7;

        in-out property <string> status_node: "-";
        in-out property <string> status_nodes: "0";
        in-out property <string> status_connected: "0";
        in-out property <string> status_failed: "0";
        in-out property <string> status_last_sync: "暂无";
        in-out property <string> message: "双方启动 agent 后，在这里添加节点和同步任务。";
        in-out property <string> selected_node_id: "";
        in-out property <string> selected_node_label: "未选择节点";
        in-out property <string> selected_task_id: "";
        in-out property <string> selected_task_label: "未选择任务";
        in-out property <bool> busy: false;

        in-out property <string> node_name: "";
        in-out property <string> node_endpoint: "";
        in-out property <string> node_fingerprint: "";

        in-out property <string> task_name: "";
        in-out property <string> source_path: "";
        in-out property <string> target_path: "";
        in-out property <bool> two_way: false;

        in property <[NodeItem]> nodes;
        in property <[TaskItem]> tasks;
        in property <[LogItem]> logs;

        callback refresh();
        callback add_node(string, string, string);
        callback add_task(string, string, string, string, bool);
        callback select_node(string);
        callback select_task(string);
        callback sync_task(string);
        callback rescan_task(string);
        callback start_watch(string);
        callback stop_watch(string);
        callback remove_task(string);
        callback remove_node(string);

        VerticalLayout {
            spacing: 0px;

            HorizontalLayout {
                padding-left: 18px;
                padding-right: 18px;
                height: 48px;
                spacing: 14px;
                Rectangle { width: 12px; height: 12px; border-radius: 6px; background: #FF5F57; }
                Rectangle { width: 12px; height: 12px; border-radius: 6px; background: #FFBD2E; }
                Rectangle { width: 12px; height: 12px; border-radius: 6px; background: #28C840; }
                VerticalLayout {
                    width: 220px;
                    spacing: 2px;
                    Text { text: "TurboSync"; font-size: 17px; font-weight: 800; color: #1D1D1F; }
                    Text { text: "Desktop Sync Console"; color: #8E8E93; font-size: 11px; }
                }
                MenuButton { text: "总览"; primary: true; width: 72px; }
                MenuButton { text: "刷新"; width: 72px; enabled: !root.busy; clicked => { root.refresh(); } }
                MenuButton { text: "同步选中"; width: 92px; enabled: !root.busy && root.selected_task_id != ""; clicked => { root.sync_task(root.selected_task_id); } }
                MenuButton { text: "监听"; width: 70px; enabled: !root.busy && root.selected_task_id != ""; clicked => { root.start_watch(root.selected_task_id); } }
                MenuButton { text: "停止"; width: 70px; enabled: !root.busy && root.selected_task_id != ""; clicked => { root.stop_watch(root.selected_task_id); } }
                Rectangle { horizontal-stretch: 1; }
                Text { text: root.message; color: #6E6E73; font-size: 12px; vertical-alignment: center; overflow: elide; }
            }

            HorizontalLayout {
                padding-left: 18px;
                padding-right: 18px;
                padding-bottom: 18px;
                spacing: 16px;

                Rectangle {
                    width: 210px;
                    background: #ECECF0;
                    border-radius: 18px;
                    VerticalLayout {
                        padding: 12px;
                        spacing: 10px;
                        Text { text: "工作台"; color: #6E6E73; font-size: 12px; font-weight: 700; }
                        SidebarItem { title: "同步总览"; subtitle: root.status_connected + " 个节点可连接"; active: true; }
                        SidebarItem { title: "当前节点"; subtitle: root.selected_node_label; active: false; }
                        SidebarItem { title: "当前任务"; subtitle: root.selected_task_label; active: false; }
                        Rectangle { height: 1px; background: #D1D1D6; }
                        Text { text: "Agent"; color: #6E6E73; font-size: 12px; font-weight: 700; }
                        Text { text: "本机节点  " + root.status_node; color: #1D1D1F; font-size: 12px; }
                        Text { text: "最近同步  " + root.status_last_sync; color: #1D1D1F; font-size: 12px; overflow: elide; }
                    }
                }

                VerticalLayout {
                    spacing: 14px;

                    HorizontalLayout {
                        spacing: 10px;
                        Metric { label: "已连节点"; value: root.status_nodes; horizontal-stretch: 1; }
                        Metric { label: "可连接"; value: root.status_connected; horizontal-stretch: 1; }
                        Metric { label: "异常"; value: root.status_failed; horizontal-stretch: 1; }
                        Metric { label: "选中任务"; value: root.selected_task_label; horizontal-stretch: 2; }
                    }

                    HorizontalLayout {
                        spacing: 14px;
                        Panel {
                            width: 360px;
                            VerticalLayout {
                        SectionTitle { title: "节点"; subtitle: "选择目标节点"; }
                        VerticalLayout {
                            padding: 12px;
                            spacing: 8px;
                            LineEdit { placeholder-text: "节点名称，例如 ubuntu-01"; text <=> root.node_name; }
                            LineEdit { placeholder-text: "同步地址，例如 127.0.0.1:38750"; text <=> root.node_endpoint; }
                            LineEdit { placeholder-text: "证书指纹，可选"; text <=> root.node_fingerprint; }
                            Button {
                                text: "添加节点";
                                enabled: !root.busy;
                                clicked => { root.add_node(root.node_name, root.node_endpoint, root.node_fingerprint); }
                            }
                        }
                        ScrollView {
                            viewport-height: 260px;
                            for node in root.nodes: NodeRow {
                                node: node;
                                selected: node.id == root.selected_node_id;
                                choose(id) => { root.select_node(id); }
                                remove(id) => { root.remove_node(id); }
                            }
                        }
                    }
                        }

                        Panel {
                            horizontal-stretch: 1;
                            VerticalLayout {
                        SectionTitle { title: "同步任务"; subtitle: root.selected_task_label; }
                        VerticalLayout {
                            padding: 12px;
                            spacing: 8px;
                            HorizontalLayout {
                                spacing: 8px;
                                LineEdit { placeholder-text: "任务名，例如 shell"; text <=> root.task_name; horizontal-stretch: 1; }
                                Button {
                                    text: root.two_way ? "双向同步" : "单向同步";
                                    clicked => { root.two_way = !root.two_way; }
                                }
                            }
                            HorizontalLayout {
                                spacing: 8px;
                                LineEdit { placeholder-text: "源目录 /home/fzf/code/shell"; text <=> root.source_path; horizontal-stretch: 1; }
                                LineEdit { placeholder-text: "目标目录 /home/ubuntu/shell"; text <=> root.target_path; horizontal-stretch: 1; }
                            }
                            HorizontalLayout {
                                Text { text: "目标节点：" + root.selected_node_label; color: #65716d; vertical-alignment: center; horizontal-stretch: 1; }
                                Button {
                                    text: "添加任务";
                                    enabled: !root.busy && root.selected_node_id != "";
                                    clicked => {
                                        root.add_task(root.task_name, root.source_path, root.selected_node_id, root.target_path, root.two_way);
                                    }
                                }
                            }
                        }
                        ScrollView {
                            viewport-height: 264px;
                            for task in root.tasks: TaskRow {
                                task: task;
                                selected: task.id == root.selected_task_id;
                                choose(id) => { root.select_task(id); }
                                sync(id) => { root.sync_task(id); }
                                rescan(id) => { root.rescan_task(id); }
                                watch(id) => { root.start_watch(id); }
                                stop_watch(id) => { root.stop_watch(id); }
                                remove(id) => { root.remove_task(id); }
                            }
                        }
                    }
                        }
                }

                    Panel {
                        horizontal-stretch: 1;
                        vertical-stretch: 1;
                        VerticalLayout {
                            SectionTitle { title: "最近活动"; subtitle: "同步结果、数据量、完成时间"; }
                            ScrollView {
                                for log in root.logs: LogRow { log: log; }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct AgentClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Deserialize)]
struct WatchStatus {
    task_id: String,
}

#[derive(Debug, Clone)]
struct Snapshot {
    status: StatusResponse,
    nodes: Vec<Node>,
    tasks: Vec<SyncTask>,
    logs: LogsResponse,
    watched: Vec<String>,
}

fn main() -> Result<()> {
    let window = MainWindow::new().context("failed to create GUI window")?;
    wire_callbacks(&window);
    refresh_async(window.as_weak());
    window.run().context("failed to run GUI")
}

fn wire_callbacks(window: &MainWindow) {
    let weak = window.as_weak();
    window.on_refresh(move || refresh_async(weak.clone()));

    let weak = window.as_weak();
    window.on_select_node(move |node_id| {
        if let Some(window) = weak.upgrade() {
            select_node(&window, node_id.as_str());
        }
    });

    let weak = window.as_weak();
    window.on_select_task(move |task_id| {
        if let Some(window) = weak.upgrade() {
            select_task(&window, task_id.as_str());
        }
    });

    let weak = window.as_weak();
    window.on_add_node(move |name, endpoint, fingerprint| {
        let request = CreateNodeRequest {
            name: name.trim().to_owned(),
            endpoint: endpoint.trim().to_owned(),
            public_key: optional_string(fingerprint.as_str()),
        };
        run_action(weak.clone(), "添加节点", move |client| async move {
            client.post_json::<_, Node>("/v1/nodes", &request).await?;
            Ok(())
        });
    });

    let weak = window.as_weak();
    window.on_add_task(move |name, source, node_id, target, two_way| {
        let direction = if two_way { "two_way" } else { "one_way" };
        let request = CreateTaskRequest {
            name: name.trim().to_owned(),
            source_path: source.trim().to_owned(),
            target_node_id: node_id.to_string(),
            target_path: target.trim().to_owned(),
            direction: direction.to_owned(),
            delete_mode: "propagate".to_owned(),
            conflict_mode: "newest_wins".to_owned(),
        };
        run_action(weak.clone(), "添加任务", move |client| async move {
            client
                .post_json::<_, SyncTask>("/v1/tasks", &request)
                .await?;
            Ok(())
        });
    });

    let weak = window.as_weak();
    window.on_sync_task(move |task_id| {
        let path = format!("/v1/tasks/{task_id}/sync");
        run_action(weak.clone(), "同步", move |client| async move {
            client.post_empty(&path).await
        });
    });

    let weak = window.as_weak();
    window.on_rescan_task(move |task_id| {
        let path = format!("/v1/tasks/{task_id}/rescan");
        run_action(weak.clone(), "扫描", move |client| async move {
            client.post_empty(&path).await
        });
    });

    let weak = window.as_weak();
    window.on_start_watch(move |task_id| {
        let path = format!("/v1/tasks/{task_id}/watch");
        run_action(weak.clone(), "启动监听", move |client| async move {
            client.post_empty(&path).await
        });
    });

    let weak = window.as_weak();
    window.on_stop_watch(move |task_id| {
        let path = format!("/v1/tasks/{task_id}/watch");
        run_action(weak.clone(), "停止监听", move |client| async move {
            client.delete(&path).await
        });
    });

    let weak = window.as_weak();
    window.on_remove_task(move |task_id| {
        let path = format!("/v1/tasks/{task_id}");
        run_action(weak.clone(), "删除任务", move |client| async move {
            client.delete(&path).await
        });
    });

    let weak = window.as_weak();
    window.on_remove_node(move |node_id| {
        let path = format!("/v1/nodes/{node_id}");
        run_action(weak.clone(), "删除节点", move |client| async move {
            client.delete(&path).await
        });
    });
}

fn optional_string(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn refresh_async(weak: slint::Weak<MainWindow>) {
    set_busy(&weak, true, "正在刷新...");
    thread::spawn(move || {
        let result = run_async(async {
            let client = AgentClient::from_config()?;
            load_snapshot(&client).await
        });
        apply_result(weak, "刷新", result);
    });
}

fn run_action<F, Fut>(weak: slint::Weak<MainWindow>, label: &'static str, action: F)
where
    F: FnOnce(AgentClient) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
{
    set_busy(&weak, true, &format!("{label}中..."));
    thread::spawn(move || {
        let result = run_async(async {
            let client = AgentClient::from_config()?;
            action(client.clone()).await?;
            load_snapshot(&client).await
        });
        apply_result(weak, label, result);
    });
}

fn run_async<T>(future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?
        .block_on(future)
}

async fn load_snapshot(client: &AgentClient) -> Result<Snapshot> {
    let status = client.get_json("/v1/status").await?;
    let nodes = client.get_json("/v1/nodes").await?;
    let tasks: Vec<SyncTask> = client.get_json("/v1/tasks").await?;
    let logs = client.get_json("/v1/logs?limit=50").await?;
    let mut watched = Vec::new();
    for task in &tasks {
        if let Ok(status) = client
            .get_json::<WatchStatus>(&format!("/v1/tasks/{}/watch", task.id))
            .await
        {
            watched.push(status.task_id);
        }
    }

    Ok(Snapshot {
        status,
        nodes,
        tasks,
        logs,
        watched,
    })
}

fn set_busy(weak: &slint::Weak<MainWindow>, busy: bool, message: &str) {
    let message = message.to_owned();
    let weak = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(window) = weak.upgrade() {
            window.set_busy(busy);
            window.set_message(message.into());
        }
    });
}

fn apply_result(weak: slint::Weak<MainWindow>, label: &'static str, result: Result<Snapshot>) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(window) = weak.upgrade() {
            match result {
                Ok(snapshot) => {
                    apply_snapshot(&window, snapshot);
                    window.set_message(format!("{label}完成").into());
                }
                Err(error) => {
                    window.set_message(format!("{label}失败：{error}").into());
                }
            }
            window.set_busy(false);
        }
    });
}

fn apply_snapshot(window: &MainWindow, snapshot: Snapshot) {
    let connected = snapshot
        .nodes
        .iter()
        .filter(|node| node.health_status == "connected")
        .count();
    let failed = snapshot
        .nodes
        .iter()
        .filter(|node| node.health_status == "failed")
        .count();

    window.set_status_node(short_id(&snapshot.status.node_id).into());
    window.set_status_nodes(snapshot.status.enabled_nodes.to_string().into());
    window.set_status_connected(connected.to_string().into());
    window.set_status_failed(failed.to_string().into());
    window.set_status_last_sync(
        format_unix_time(snapshot.status.last_sync_at.as_deref(), "暂无").into(),
    );

    let current_selected = window.get_selected_node_id().to_string();
    window.set_nodes(model(snapshot.nodes.iter().map(node_item).collect()));
    window.set_tasks(model(
        snapshot
            .tasks
            .iter()
            .map(|task| task_item(task, &snapshot.nodes, &snapshot.watched))
            .collect(),
    ));
    window.set_logs(model(snapshot.logs.runs.iter().map(log_item).collect()));

    if snapshot
        .nodes
        .iter()
        .any(|node| node.id == current_selected)
    {
        select_node(window, &current_selected);
    } else if let Some(node) = snapshot.nodes.first() {
        select_node(window, &node.id);
    } else {
        window.set_selected_node_id("".into());
        window.set_selected_node_label("未选择节点".into());
    }

    let current_task = window.get_selected_task_id().to_string();
    if snapshot.tasks.iter().any(|task| task.id == current_task) {
        select_task(window, &current_task);
    } else if let Some(task) = snapshot.tasks.first() {
        select_task(window, &task.id);
    } else {
        window.set_selected_task_id("".into());
        window.set_selected_task_label("未选择任务".into());
    }
}

fn select_node(window: &MainWindow, node_id: &str) {
    let nodes = window.get_nodes();
    for row in nodes.iter() {
        if row.id == node_id {
            window.set_selected_node_id(row.id.clone());
            window.set_selected_node_label(format!("{} · {}", row.name, row.endpoint).into());
            return;
        }
    }
    window.set_selected_node_id("".into());
    window.set_selected_node_label("未选择节点".into());
}

fn select_task(window: &MainWindow, task_id: &str) {
    let tasks = window.get_tasks();
    for row in tasks.iter() {
        if row.id == task_id {
            window.set_selected_task_id(row.id.clone());
            window.set_selected_task_label(format!("{} · {}", row.name, row.short_id).into());
            return;
        }
    }
    window.set_selected_task_id("".into());
    window.set_selected_task_label("未选择任务".into());
}

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(Rc::new(VecModel::from(items)))
}

fn node_item(node: &Node) -> NodeItem {
    NodeItem {
        id: node.id.clone().into(),
        name: node.name.clone().into(),
        endpoint: node.endpoint.clone().into(),
        health: health_label(&node.health_status).into(),
        detail: node
            .health_message
            .clone()
            .unwrap_or_else(|| format!("节点 {}", short_id(&node.id)))
            .into(),
    }
}

fn task_item(task: &SyncTask, nodes: &[Node], watched: &[String]) -> TaskItem {
    let node = nodes.iter().find(|node| node.id == task.target_node_id);
    let node_label = node
        .map(|node| format!("{} {}", node.name, short_id(&node.id)))
        .unwrap_or_else(|| short_id(&task.target_node_id));
    let direction = if task.direction == "two_way" {
        "双向同步"
    } else {
        "单向同步"
    };
    let watch = if watched.iter().any(|id| id == &task.id) {
        "监听中"
    } else {
        "未监听"
    };

    TaskItem {
        id: task.id.clone().into(),
        short_id: short_id(&task.id).into(),
        name: task.name.clone().into(),
        route: format!("{} → {}:{}", task.source_path, node_label, task.target_path).into(),
        meta: format!("{direction} · {} · {watch}", task.conflict_mode).into(),
    }
}

fn log_item(run: &turbosync_core::models::SyncRun) -> LogItem {
    LogItem {
        status: status_label(&run.status).into(),
        trigger: trigger_label(&run.trigger_kind).into(),
        summary: format!(
            "变更={} · 失败={} · 数据={}",
            run.files_changed,
            run.files_failed,
            format_bytes(run.bytes_sent)
        )
        .into(),
        time: format_unix_time(run.finished_at.as_deref(), "进行中").into(),
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

fn health_label(status: &str) -> &'static str {
    match status {
        "connected" => "已连接",
        "failed" => "连接失败",
        _ => "未检测",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "success" => "成功",
        "failed" => "失败",
        "running" => "运行中",
        _ => "未知",
    }
}

fn trigger_label(trigger: &str) -> &'static str {
    match trigger {
        "manual_sync" => "手动同步",
        "manual_rescan" => "手动扫描",
        "watcher" => "文件变化",
        _ => "其他",
    }
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
        self.expect_success(response)
            .await?
            .json()
            .await
            .context("响应解析失败")
    }

    async fn expect_status(&self, response: reqwest::Response) -> Result<()> {
        self.expect_success(response).await?;
        Ok(())
    }

    async fn expect_success(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if body.trim().is_empty() {
            anyhow::bail!("agent 请求失败：{status}");
        }
        anyhow::bail!("agent 请求失败：{status}: {body}");
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}
