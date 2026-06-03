use anyhow::Result;

pub async fn run_foreground() -> Result<()> {
    tracing::info!("TurboSync agent skeleton started");
    println!("TurboSync agent skeleton started.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn foreground_agent_exits_cleanly_in_skeleton_mode() {
        run_foreground().await.unwrap();
    }
}
