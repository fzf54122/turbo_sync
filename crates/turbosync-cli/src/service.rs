use std::path::PathBuf;

use anyhow::{Context, Result};

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME environment variable not set"))
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::Command;

    const SERVICE_NAME: &str = "turbosync-agent";
    const SERVICE_DIR: &str = ".config/systemd/user";
    const SERVICE_FILE: &str = "turbosync-agent.service";

    fn service_dir() -> PathBuf {
        home_dir().join(SERVICE_DIR)
    }

    fn service_path() -> PathBuf {
        service_dir().join(SERVICE_FILE)
    }

    pub fn install() -> Result<()> {
        let binary_path =
            std::env::current_exe().context("failed to resolve current binary path")?;
        let unit = format!(
            "[Unit]\n\
             Description=TurboSync Agent\n\
             After=network-online.target\n\
             Wants=network-online.target\n\n\
             [Service]\n\
             ExecStart={} agent run\n\
             Restart=on-failure\n\
             RestartSec=5\n\n\
             [Install]\n\
             WantedBy=default.target\n",
            binary_path.display()
        );

        std::fs::create_dir_all(service_dir())
            .context("failed to create systemd user directory")?;
        std::fs::write(service_path(), &unit).context("failed to write systemd service file")?;

        let status = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .context("failed to reload systemd user daemon")?;
        anyhow::ensure!(status.success(), "systemctl --user daemon-reload failed");

        let status = Command::new("systemctl")
            .args(["--user", "enable", "--now", SERVICE_NAME])
            .status()
            .context("failed to enable turbosync-agent service")?;
        anyhow::ensure!(status.success(), "systemctl --user enable --now failed");

        println!("Service installed and started.");
        println!("  systemctl --user status {SERVICE_NAME}");
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let path = service_path();
        if !path.exists() {
            println!("Service is not installed.");
            return Ok(());
        }

        let _ = Command::new("systemctl")
            .args(["--user", "stop", SERVICE_NAME])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", SERVICE_NAME])
            .status();

        std::fs::remove_file(&path).context("failed to remove service file")?;

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        println!("Service uninstalled.");
        Ok(())
    }

    pub fn status() -> Result<()> {
        let path = service_path();
        if !path.exists() {
            println!("Service is not installed.");
            return Ok(());
        }

        let output = Command::new("systemctl")
            .args(["--user", "status", SERVICE_NAME])
            .output()
            .context("failed to query service status")?;

        let text = String::from_utf8_lossy(&output.stdout);
        if text.is_empty() {
            let err = String::from_utf8_lossy(&output.stderr);
            println!("{err}");
        } else {
            println!("{text}");
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::process::Command;

    const LABEL: &str = "dev.turbosync.agent";
    const PLIST_DIR: &str = "Library/LaunchAgents";
    const PLIST_FILE: &str = "dev.turbosync.agent.plist";

    fn plist_dir() -> PathBuf {
        home_dir().join(PLIST_DIR)
    }

    fn plist_path() -> PathBuf {
        plist_dir().join(PLIST_FILE)
    }

    pub fn install() -> Result<()> {
        let binary_path =
            std::env::current_exe().context("failed to resolve current binary path")?;
        let log_dir = home_dir().join("Library/Logs");
        std::fs::create_dir_all(&log_dir).ok();
        let log_path = log_dir.join("turbosync-agent.log");

        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \x20\x20\x20\x20<key>Label</key>\n\
             \x20\x20\x20\x20<string>{LABEL}</string>\n\
             \x20\x20\x20\x20<key>ProgramArguments</key>\n\
             \x20\x20\x20\x20<array>\n\
             \x20\x20\x20\x20\x20\x20\x20\x20<string>{}</string>\n\
             \x20\x20\x20\x20\x20\x20\x20\x20<string>agent</string>\n\
             \x20\x20\x20\x20\x20\x20\x20\x20<string>run</string>\n\
             \x20\x20\x20\x20</array>\n\
             \x20\x20\x20\x20<key>RunAtLoad</key>\n\
             \x20\x20\x20\x20<true/>\n\
             \x20\x20\x20\x20<key>KeepAlive</key>\n\
             \x20\x20\x20\x20<true/>\n\
             \x20\x20\x20\x20<key>StandardOutPath</key>\n\
             \x20\x20\x20\x20<string>{log_path}</string>\n\
             \x20\x20\x20\x20<key>StandardErrorPath</key>\n\
             \x20\x20\x20\x20<string>{log_path}</string>\n\
             </dict>\n\
             </plist>\n",
            binary_path.display(),
            log_path = log_path.display(),
        );

        std::fs::create_dir_all(plist_dir()).context("failed to create LaunchAgents directory")?;
        std::fs::write(plist_path(), &plist).context("failed to write launchd plist file")?;

        let status = Command::new("launchctl")
            .args(["bootstrap", "gui/501", &plist_path().display().to_string()])
            .status()
            .context("failed to bootstrap launchd service")?;
        anyhow::ensure!(status.success(), "launchctl bootstrap failed");

        println!("Service installed and started.");
        println!("  launchctl list | grep {LABEL}");
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let path = plist_path();
        if !path.exists() {
            println!("Service is not installed.");
            return Ok(());
        }

        let _ = Command::new("launchctl")
            .args(["bootout", "gui/501", &path.display().to_string()])
            .status();

        std::fs::remove_file(&path).context("failed to remove plist file")?;
        println!("Service uninstalled.");
        Ok(())
    }

    pub fn status() -> Result<()> {
        let path = plist_path();
        if !path.exists() {
            println!("Service is not installed.");
            return Ok(());
        }

        let output = Command::new("launchctl")
            .args(["list"])
            .output()
            .context("failed to query launchd services")?;

        let text = String::from_utf8_lossy(&output.stdout);
        let found = text.lines().any(|line| line.contains(LABEL));
        if found {
            println!("Service is running.");
        } else {
            println!("Service is installed but not running.");
        }
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    use super::*;

    pub fn install() -> Result<()> {
        anyhow::bail!("service install is only supported on Linux and macOS");
    }

    pub fn uninstall() -> Result<()> {
        anyhow::bail!("service uninstall is only supported on Linux and macOS");
    }

    pub fn status() -> Result<()> {
        anyhow::bail!("service status is only supported on Linux and macOS");
    }
}

pub fn install() -> Result<()> {
    platform::install()
}

pub fn uninstall() -> Result<()> {
    platform::uninstall()
}

pub fn status() -> Result<()> {
    platform::status()
}
