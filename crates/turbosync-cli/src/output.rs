use turbosync_core::models::{Node, StatusResponse, SyncTask};

pub fn print_status(status: &StatusResponse) {
    println!("Status: {}", status.status);
    println!("Node ID: {}", status.node_id);
    println!("Enabled nodes: {}", status.enabled_nodes);
    println!("Enabled tasks: {}", status.enabled_tasks);
    println!("Pending events: {}", status.pending_events);
    println!(
        "Last sync: {}",
        status.last_sync_at.as_deref().unwrap_or("never")
    );
}

pub fn print_node(node: &Node) {
    println!("{}\t{}\t{}", node.id, node.name, node.endpoint);
}

pub fn print_nodes(nodes: &[Node]) {
    if nodes.is_empty() {
        println!("No nodes configured.");
        return;
    }

    for node in nodes {
        print_node(node);
    }
}

pub fn print_task(task: &SyncTask) {
    println!(
        "{}\t{}\t{} -> {}:{}",
        task.id, task.name, task.source_path, task.target_node_id, task.target_path
    );
}

pub fn print_tasks(tasks: &[SyncTask]) {
    if tasks.is_empty() {
        println!("No sync tasks configured.");
        return;
    }

    for task in tasks {
        print_task(task);
    }
}
