use turbosync_core::models::{
    LogsResponse, Node, RescanResponse, StatusResponse, SyncResponse, SyncTask,
};

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

pub fn print_rescan_response(response: &RescanResponse) {
    println!("Run: {}", response.run.id);
    println!("Status: {}", response.run.status);
    println!("Files scanned: {}", response.files_scanned);
    println!("Files changed: {}", response.files_changed);
}

pub fn print_sync_response(response: &SyncResponse) {
    let failed = response
        .operations
        .iter()
        .filter(|operation| operation.status == "failed")
        .count();

    println!("Run: {}", response.run.id);
    println!("Status: {}", response.run.status);
    println!("Operations: {}", response.operations.len());
    println!("Failed: {failed}");
    println!("Bytes copied: {}", response.run.bytes_sent);
}

pub fn print_logs(response: &LogsResponse) {
    if response.runs.is_empty() {
        println!("No sync runs recorded.");
        return;
    }

    for run in &response.runs {
        println!(
            "{}\t{}\t{}\tchanged={}\tfailed={}\tfinished={}",
            run.id,
            run.trigger_kind,
            run.status,
            run.files_changed,
            run.files_failed,
            run.finished_at.as_deref().unwrap_or("-")
        );
    }
}
