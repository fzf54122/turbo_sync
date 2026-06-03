use anyhow::Result;
use clap::{Parser, Subcommand};

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

async fn run_with_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init => println!("TurboSync init is not implemented yet."),
        Command::Agent { command } => match command {
            AgentCommand::Run => turbosync_agent::run_foreground().await?,
        },
        Command::Status => println!("TurboSync status is not implemented yet."),
        Command::Node { command } => match command {
            NodeCommand::Add { name, endpoint } => {
                println!("TurboSync node add is not implemented yet: {name} {endpoint}")
            }
            NodeCommand::List => println!("TurboSync node list is not implemented yet."),
            NodeCommand::Remove { node_id } => {
                println!("TurboSync node remove is not implemented yet: {node_id}")
            }
        },
        Command::Task { command } => match command {
            TaskCommand::Add {
                name,
                source,
                target_node,
                target_path,
            } => println!(
                "TurboSync task add is not implemented yet: {name} {source} {target_node} {target_path}"
            ),
            TaskCommand::List => println!("TurboSync task list is not implemented yet."),
            TaskCommand::Remove { task_id } => {
                println!("TurboSync task remove is not implemented yet: {task_id}")
            }
        },
        Command::Sync { task_id } => println!("TurboSync sync is not implemented yet: {task_id}"),
        Command::Rescan { task_id } => println!("TurboSync rescan is not implemented yet: {task_id}"),
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
}
