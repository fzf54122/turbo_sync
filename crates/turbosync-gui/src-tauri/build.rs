fn main() {
    // Extract version from git tag if available (e.g., v0.1.2 -> 0.1.2)
    if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
    {
        if output.status.success() {
            if let Ok(tag) = String::from_utf8(output.stdout) {
                let version = tag.trim().trim_start_matches('v');
                println!("cargo:rustc-env=CARGO_PKG_VERSION={}", version);
            }
        }
    }

    tauri_build::build();
}
