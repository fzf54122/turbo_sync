mod client;
mod output;

use anyhow::Result;
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
    /// Run or manage the local agent.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Show local agent status.
    Status,
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
    /// Show recent sync logs.
    Logs {
        /// Maximum number of log entries to show.
        #[arg(long, default_value_t = 50)]
        limit: u16,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// Run the agent in the foreground.
    Run,
}

#[derive(Debug, Subcommand)]
enum NodeCommand {
    /// Add a sync node.
    Add { name: String, endpoint: String },
    /// List sync nodes.
    List,
    /// Remove a sync node.
    Remove { node_id: String },
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

async fn run_node_command(command: NodeCommand) -> Result<()> {
    let client = AgentClient::from_config()?;

    match command {
        NodeCommand::Add { name, endpoint } => {
            let node = client
                .add_node(&CreateNodeRequest {
                    name,
                    endpoint,
                    public_key: None,
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
        } => {
            let task = client
                .add_task(&CreateTaskRequest::one_way(
                    name,
                    source,
                    target_node,
                    target_path,
                ))
                .await?;
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
        },
        Command::Status => {
            let client = AgentClient::from_config()?;
            let status = client.status().await?;
            output::print_status(&status);
        }
        Command::Node { command } => run_node_command(command).await?,
        Command::Task { command } => run_task_command(command).await?,
        Command::Sync { task_id } => println!("TurboSync sync is not implemented yet: {task_id}"),
        Command::Rescan { task_id } => {
            println!("TurboSync rescan is not implemented yet: {task_id}")
        }
        Command::Logs { limit } => println!("TurboSync logs is not implemented yet: {limit}"),
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
            "init", "agent", "status", "node", "task", "sync", "rescan", "logs",
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
        };

        let app_config = init_with_paths(&paths).await.unwrap();

        assert_eq!(app_config.db_path, paths.db_file);
        assert!(paths.config_file.exists());
        assert!(paths.db_file.exists());
    }
}
