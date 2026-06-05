mod client;
mod output;
mod service;

use std::{
    process::{Child, Stdio},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use turbosync_core::{config, models::*};

use crate::client::AgentClient;

#[derive(Debug, Parser)]
#[command(name = "tsync", version, about = "TurboSync command line interface")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize local TurboSync state.
    Init,
    /// Run or manage the local background sync service.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Show local agent status.
    Status,
    /// Open the interactive terminal dashboard; starts the local sync service if needed.
    Dashboard,
    /// Manage sync nodes.
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
    /// Manage sync tasks.
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// Manually run a sync task.
    Sync {
        /// Sync task id.
        task_id: String,
    },
    /// Rescan a sync task source directory.
    Rescan {
        /// Sync task id.
        task_id: String,
    },
    /// Manage file watching for a task.
    Watch {
        task_id: String,
        #[command(subcommand)]
        command: WatchCommand,
    },
    /// Show recent sync logs.
    Logs {
        /// Maximum number of log entries to show.
        #[arg(long, default_value_t = 50)]
        limit: u16,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// Run the local sync service in the foreground.
    Run,
    /// Manage the background sync service.
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    /// Install and start the background service.
    Install,
    /// Stop and remove the background service.
    Uninstall,
    /// Show service status.
    Status,
}

#[derive(Debug, Subcommand)]
enum NodeCommand {
    /// Add a sync node.
    Add {
        name: String,
        endpoint: String,
        /// Expected TLS certificate fingerprint (SHA256 hex).
        #[arg(long)]
        cert_fingerprint: Option<String>,
    },
    /// List sync nodes.
    List,
    /// Remove a sync node.
    Remove { node_id: String },
}

#[derive(Debug, Subcommand)]
enum WatchCommand {
    /// Start watching a task for file changes.
    Start,
    /// Stop watching a task.
    Stop,
    /// Check if a task is being watched.
    Status,
}

#[derive(Debug, Subcommand)]
enum TaskCommand {
    /// Add a sync task.
    Add {
        name: String,
        #[arg(long)]
        source: String,
        #[arg(long = "target-node")]
        target_node: String,
        #[arg(long = "target-path")]
        target_path: String,
        /// Sync direction: "one_way" (default) or "two_way".
        #[arg(long, default_value = "one_way")]
        direction: String,
        /// Conflict resolution mode for two_way sync: "newest_wins" (default) or "manual".
        #[arg(long = "conflict-mode", default_value = "newest_wins")]
        conflict_mode: String,
    },
    /// List sync tasks.
    List,
    /// Remove a sync task.
    Remove { task_id: String },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_with_cli(cli).await
}

async fn init() -> Result<()> {
    let paths = config::resolve_paths()?;
    let config = init_with_paths(&paths).await?;

    println!("TurboSync initialized.");
    println!("Config: {}", paths.config_file.display());
    println!("Database: {}", config.db_path.display());

    Ok(())
}

async fn init_with_paths(paths: &config::ConfigPaths) -> Result<config::AppConfig> {
    let config = config::init_config_at(paths)?;
    turbosync_storage::initialize_database(&config.db_path).await?;
    Ok(config)
}

async fn ensure_local_agent_running() -> Result<Option<Child>> {
    if let Ok(client) = AgentClient::from_config() {
        if client.health_check().await.is_ok() {
            return Ok(None);
        }
    }

    let paths = config::resolve_paths()?;
    if !paths.config_file.exists() {
        init_with_paths(&paths).await?;
    }

    let current_exe = std::env::current_exe().context("failed to locate current tsync binary")?;
    let mut child = std::process::Command::new(current_exe)
        .args(["agent", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start local sync service")?;

    for _ in 0..30 {
        if let Ok(client) = AgentClient::from_config() {
            if client.health_check().await.is_ok() {
                return Ok(Some(child));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = child.kill();
    let _ = child.wait();
    anyhow::bail!("local sync service did not become ready");
}

fn stop_auto_started_agent(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

async fn run_node_command(command: NodeCommand) -> Result<()> {
    let client = AgentClient::from_config()?;

    match command {
        NodeCommand::Add {
            name,
            endpoint,
            cert_fingerprint,
        } => {
            let node = client
                .add_node(&CreateNodeRequest {
                    name,
                    endpoint,
                    public_key: cert_fingerprint,
                })
                .await?;
            output::print_node(&node);
        }
        NodeCommand::List => {
            let nodes = client.list_nodes().await?;
            output::print_nodes(&nodes);
        }
        NodeCommand::Remove { node_id } => {
            if client.remove_node(&node_id).await? {
                println!("Removed node: {node_id}");
            } else {
                println!("Node not found: {node_id}");
            }
        }
    }

    Ok(())
}

async fn run_task_command(command: TaskCommand) -> Result<()> {
    let client = AgentClient::from_config()?;

    match command {
        TaskCommand::Add {
            name,
            source,
            target_node,
            target_path,
            direction,
            conflict_mode,
        } => {
            let request = if direction == "two_way" {
                CreateTaskRequest::two_way(name, source, target_node, target_path, conflict_mode)
            } else {
                CreateTaskRequest::one_way(name, source, target_node, target_path)
            };
            let task = client.add_task(&request).await?;
            output::print_task(&task);
        }
        TaskCommand::List => {
            let tasks = client.list_tasks().await?;
            output::print_tasks(&tasks);
        }
        TaskCommand::Remove { task_id } => {
            if client.remove_task(&task_id).await? {
                println!("Removed task: {task_id}");
            } else {
                println!("Task not found: {task_id}");
            }
        }
    }

    Ok(())
}

async fn run_with_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init => init().await?,
        Command::Agent { command } => match command {
            AgentCommand::Run => turbosync_agent::run_foreground().await?,
            AgentCommand::Service { command } => match command {
                ServiceCommand::Install => service::install()?,
                ServiceCommand::Uninstall => service::uninstall()?,
                ServiceCommand::Status => service::status()?,
            },
        },
        Command::Status => {
            let client = AgentClient::from_config()?;
            let status = client.status().await?;
            output::print_status(&status);
        }
        Command::Dashboard => {
            let mut auto_started_agent = ensure_local_agent_running().await?;
            let result = turbosync_tui::run().await;
            if let Some(child) = auto_started_agent.as_mut() {
                stop_auto_started_agent(child);
            }
            result?;
        }
        Command::Node { command } => run_node_command(command).await?,
        Command::Task { command } => run_task_command(command).await?,
        Command::Sync { task_id } => {
            let client = AgentClient::from_config()?;
            let response = client.sync_task(&task_id).await?;
            output::print_sync_response(&response);
        }
        Command::Rescan { task_id } => {
            let client = AgentClient::from_config()?;
            let response = client.rescan_task(&task_id).await?;
            output::print_rescan_response(&response);
        }
        Command::Watch { task_id, command } => match command {
            WatchCommand::Start => {
                let client = AgentClient::from_config()?;
                let status = client.start_watch(&task_id).await?;
                output::print_watch_status(&status);
            }
            WatchCommand::Stop => {
                let client = AgentClient::from_config()?;
                if client.stop_watch(&task_id).await? {
                    println!("Stopped watching task: {task_id}");
                } else {
                    println!("Task was not being watched: {task_id}");
                }
            }
            WatchCommand::Status => {
                let client = AgentClient::from_config()?;
                match client.get_watch_status(&task_id).await {
                    Ok(status) => output::print_watch_status(&status),
                    Err(_) => println!("Task is not being watched: {task_id}"),
                }
            }
        },
        Command::Logs { limit } => {
            let client = AgentClient::from_config()?;
            let response = client.logs(limit).await?;
            output::print_logs(&response);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn help_lists_mvp_commands() {
        let help = Cli::command().render_long_help().to_string();

        for command in [
            "init",
            "agent",
            "status",
            "dashboard",
            "node",
            "task",
            "sync",
            "rescan",
            "watch",
            "logs",
        ] {
            assert!(help.contains(command), "missing command: {command}");
        }
    }

    #[tokio::test]
    async fn init_creates_config_and_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let paths = config::ConfigPaths {
            config_file: temp_dir.path().join("config/config.toml"),
            data_dir: temp_dir.path().join("data"),
            db_file: temp_dir.path().join("data/turbosync.db"),
            cert_dir: temp_dir.path().join("data/cert"),
        };

        let app_config = init_with_paths(&paths).await.unwrap();

        assert_eq!(app_config.db_path, paths.db_file);
        assert!(paths.config_file.exists());
        assert!(paths.db_file.exists());
    }
}
