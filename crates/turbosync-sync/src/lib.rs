use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

use anyhow::{bail, Context, Result};
use turbosync_core::models::FileIndexEntry;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEntry {
    pub relative_path: String,
    pub file_kind: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub entries: Vec<ScannedEntry>,
    pub files_scanned: i64,
    pub bytes_scanned: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlannedOperationKind {
    CreateFile,
    UpdateFile,
    DeleteFile,
    CreateDir,
    DeleteDir,
}

impl PlannedOperationKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateFile => "create_file",
            Self::UpdateFile => "update_file",
            Self::DeleteFile => "delete_file",
            Self::CreateDir => "create_dir",
            Self::DeleteDir => "delete_dir",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOperation {
    pub relative_path: String,
    pub kind: PlannedOperationKind,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedOperation {
    pub relative_path: String,
    pub kind: PlannedOperationKind,
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedOperation {
    pub relative_path: String,
    pub kind: PlannedOperationKind,
    pub size_bytes: Option<i64>,
    pub error_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplySummary {
    pub succeeded: Vec<AppliedOperation>,
    pub failed: Vec<FailedOperation>,
    pub bytes_copied: i64,
}

pub fn scan_source(source_path: &Path) -> Result<ScanResult> {
    if !source_path.is_dir() {
        bail!("source path is not a directory: {}", source_path.display());
    }

    let mut entries = Vec::new();
    let mut files_scanned = 0;
    let mut bytes_scanned = 0;

    for entry in WalkDir::new(source_path)
        .follow_links(false)
        .sort_by_file_name()
    {
        let entry = entry.context("failed to walk source directory")?;
        let path = entry.path();

        if path == source_path {
            continue;
        }

        if entry.file_type().is_symlink() {
            continue;
        }

        let relative_path = relative_path(source_path, path)?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;

        if metadata.is_dir() {
            entries.push(ScannedEntry {
                relative_path,
                file_kind: "dir".to_owned(),
                size_bytes: None,
                modified_at: modified_at_text(&metadata),
                content_hash: None,
            });
        } else if metadata.is_file() {
            let size_bytes = i64::try_from(metadata.len()).context("file size exceeds i64")?;
            let content_hash = hash_file(path)?;
            files_scanned += 1;
            bytes_scanned += size_bytes;
            entries.push(ScannedEntry {
                relative_path,
                file_kind: "file".to_owned(),
                size_bytes: Some(size_bytes),
                modified_at: modified_at_text(&metadata),
                content_hash: Some(content_hash),
            });
        }
    }

    Ok(ScanResult {
        entries,
        files_scanned,
        bytes_scanned,
    })
}

#[must_use]
pub fn plan_changes(
    previous: &[FileIndexEntry],
    current: &[ScannedEntry],
) -> Vec<PlannedOperation> {
    let previous_by_path: BTreeMap<&str, &FileIndexEntry> = previous
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect();
    let current_by_path: BTreeMap<&str, &ScannedEntry> = current
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect();
    let mut operations = Vec::new();
    let mut deleted_paths = BTreeSet::new();

    for current_entry in current_by_path.values() {
        match previous_by_path.get(current_entry.relative_path.as_str()) {
            None => operations.push(create_operation(current_entry)),
            Some(previous_entry) if previous_entry.deleted => {
                operations.push(create_operation(current_entry));
            }
            Some(previous_entry) if previous_entry.file_kind != current_entry.file_kind => {
                operations.push(delete_operation(previous_entry));
                operations.push(create_operation(current_entry));
                deleted_paths.insert(previous_entry.relative_path.as_str());
            }
            Some(previous_entry) if previous_entry.last_synced_at.is_none() => {
                operations.push(create_operation(current_entry));
            }
            Some(previous_entry) if is_changed(previous_entry, current_entry) => {
                operations.push(update_operation(current_entry));
            }
            Some(_) => {}
        }
    }

    let mut delete_operations: Vec<PlannedOperation> = previous_by_path
        .values()
        .filter(|entry| !entry.deleted)
        .filter(|entry| !deleted_paths.contains(entry.relative_path.as_str()))
        .filter(|entry| !current_by_path.contains_key(entry.relative_path.as_str()))
        .map(|entry| delete_operation(entry))
        .collect();

    delete_operations.sort_by(|left, right| {
        path_depth(&right.relative_path).cmp(&path_depth(&left.relative_path))
    });
    operations.extend(delete_operations);
    operations
}

pub fn apply_local_operations(
    source_root: &Path,
    target_root: &Path,
    operations: &[PlannedOperation],
) -> Result<ApplySummary> {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let mut bytes_copied = 0;

    for operation in operations {
        match apply_operation(source_root, target_root, operation) {
            Ok(bytes) => {
                bytes_copied += bytes;
                succeeded.push(AppliedOperation {
                    relative_path: operation.relative_path.clone(),
                    kind: operation.kind,
                    size_bytes: operation.size_bytes,
                });
            }
            Err(error) => failed.push(FailedOperation {
                relative_path: operation.relative_path.clone(),
                kind: operation.kind,
                size_bytes: operation.size_bytes,
                error_message: error.to_string(),
            }),
        }
    }

    Ok(ApplySummary {
        succeeded,
        failed,
        bytes_copied,
    })
}

fn apply_operation(
    source_root: &Path,
    target_root: &Path,
    operation: &PlannedOperation,
) -> Result<i64> {
    let source_path = safe_join(source_root, &operation.relative_path)?;
    let target_path = safe_join(target_root, &operation.relative_path)?;

    match operation.kind {
        PlannedOperationKind::CreateDir => {
            fs::create_dir_all(&target_path)
                .with_context(|| format!("failed to create directory {}", target_path.display()))?;
            Ok(0)
        }
        PlannedOperationKind::CreateFile | PlannedOperationKind::UpdateFile => {
            let metadata = fs::symlink_metadata(&source_path).with_context(|| {
                format!("failed to read source metadata {}", source_path.display())
            })?;
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                bail!(
                    "source path is not a regular file: {}",
                    source_path.display()
                );
            }
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            let bytes = fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
            i64::try_from(bytes).context("copied byte count exceeds i64")
        }
        PlannedOperationKind::DeleteFile => {
            if target_path.exists() {
                fs::remove_file(&target_path)
                    .with_context(|| format!("failed to delete file {}", target_path.display()))?;
            }
            Ok(0)
        }
        PlannedOperationKind::DeleteDir => {
            if target_path.exists() {
                fs::remove_dir(&target_path).with_context(|| {
                    format!("failed to delete empty directory {}", target_path.display())
                })?;
            }
            Ok(0)
        }
    }
}

fn safe_join(root: &Path, relative_path: &str) -> Result<PathBuf> {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        bail!("relative path must not be absolute: {relative_path}");
    }

    let mut joined = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("relative path escapes sync root: {relative_path}");
            }
        }
    }

    Ok(joined)
}

fn create_operation(entry: &ScannedEntry) -> PlannedOperation {
    PlannedOperation {
        relative_path: entry.relative_path.clone(),
        kind: if entry.file_kind == "dir" {
            PlannedOperationKind::CreateDir
        } else {
            PlannedOperationKind::CreateFile
        },
        size_bytes: entry.size_bytes,
    }
}

fn update_operation(entry: &ScannedEntry) -> PlannedOperation {
    PlannedOperation {
        relative_path: entry.relative_path.clone(),
        kind: PlannedOperationKind::UpdateFile,
        size_bytes: entry.size_bytes,
    }
}

fn delete_operation(entry: &FileIndexEntry) -> PlannedOperation {
    PlannedOperation {
        relative_path: entry.relative_path.clone(),
        kind: if entry.file_kind == "dir" {
            PlannedOperationKind::DeleteDir
        } else {
            PlannedOperationKind::DeleteFile
        },
        size_bytes: entry.size_bytes,
    }
}

fn is_changed(previous: &FileIndexEntry, current: &ScannedEntry) -> bool {
    current.file_kind == "file"
        && (previous.content_hash != current.content_hash
            || previous.size_bytes != current.size_bytes)
}

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("failed to normalize path {}", path.display()))?;
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    if parts.is_empty() {
        bail!("empty relative path for {}", path.display());
    }

    Ok(parts.join("/"))
}

fn path_depth(relative_path: &str) -> usize {
    relative_path.split('/').count()
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open file for hashing {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read file for hashing {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn modified_at_text(metadata: &fs::Metadata) -> Option<String> {
    metadata
        .modified()
        .ok()
        .and_then(|modified_at| modified_at.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_source_indexes_files_with_relative_paths_and_hashes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        fs::create_dir_all(source.join("notes")).unwrap();
        fs::write(source.join("notes/a.txt"), b"hello\n").unwrap();

        let result = scan_source(&source).unwrap();
        let file = result
            .entries
            .iter()
            .find(|entry| entry.relative_path == "notes/a.txt")
            .unwrap();

        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.bytes_scanned, 6);
        assert_eq!(file.file_kind, "file");
        assert_eq!(file.size_bytes, Some(6));
        assert_eq!(
            file.content_hash.as_deref(),
            Some(blake3::hash(b"hello\n").to_hex().as_str())
        );
        assert!(result
            .entries
            .iter()
            .any(|entry| entry.relative_path == "notes" && entry.file_kind == "dir"));
    }

    #[test]
    fn scan_source_skips_root_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        fs::create_dir_all(&source).unwrap();

        let result = scan_source(&source).unwrap();

        assert!(result.entries.is_empty());
    }

    #[test]
    fn plan_changes_detects_create_update_delete() {
        let previous = vec![
            file_index_entry("a.txt", "file", Some("old"), Some(3), false),
            file_index_entry("old.txt", "file", Some("old"), Some(4), false),
            file_index_entry("restored.txt", "file", Some("old"), Some(5), true),
        ];
        let current = vec![
            scanned_entry("a.txt", "file", Some("new"), Some(3)),
            scanned_entry("new.txt", "file", Some("hash"), Some(7)),
            scanned_entry("restored.txt", "file", Some("old"), Some(5)),
        ];

        let operations = plan_changes(&previous, &current);

        assert!(operations.contains(&planned("a.txt", PlannedOperationKind::UpdateFile, Some(3))));
        assert!(operations.contains(&planned(
            "new.txt",
            PlannedOperationKind::CreateFile,
            Some(7)
        )));
        assert!(operations.contains(&planned(
            "old.txt",
            PlannedOperationKind::DeleteFile,
            Some(4)
        )));
        assert!(operations.contains(&planned(
            "restored.txt",
            PlannedOperationKind::CreateFile,
            Some(5)
        )));
    }

    #[test]
    fn plan_changes_syncs_current_entries_without_last_synced_at() {
        let mut previous = file_index_entry("a.txt", "file", Some("hash"), Some(5), false);
        previous.last_synced_at = None;
        let current = vec![scanned_entry("a.txt", "file", Some("hash"), Some(5))];

        let operations = plan_changes(&[previous], &current);

        assert_eq!(
            operations,
            vec![planned("a.txt", PlannedOperationKind::CreateFile, Some(5))]
        );
    }

    #[test]
    fn apply_local_operations_copies_created_and_updated_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        fs::create_dir_all(source.join("notes")).unwrap();
        fs::write(source.join("notes/a.txt"), b"hello").unwrap();

        let summary = apply_local_operations(
            &source,
            &target,
            &[
                planned("notes", PlannedOperationKind::CreateDir, None),
                planned("notes/a.txt", PlannedOperationKind::CreateFile, Some(5)),
            ],
        )
        .unwrap();

        assert!(summary.failed.is_empty());
        assert_eq!(summary.bytes_copied, 5);
        assert_eq!(fs::read(target.join("notes/a.txt")).unwrap(), b"hello");
    }

    #[test]
    fn apply_local_operations_deletes_only_planned_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("tracked.txt"), b"tracked").unwrap();
        fs::write(target.join("untracked.txt"), b"untracked").unwrap();

        let summary = apply_local_operations(
            &source,
            &target,
            &[planned(
                "tracked.txt",
                PlannedOperationKind::DeleteFile,
                Some(7),
            )],
        )
        .unwrap();

        assert!(summary.failed.is_empty());
        assert!(!target.join("tracked.txt").exists());
        assert!(target.join("untracked.txt").exists());
    }

    #[test]
    fn apply_local_operations_rejects_path_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");
        let target = temp_dir.path().join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();

        let summary = apply_local_operations(
            &source,
            &target,
            &[planned(
                "../escape.txt",
                PlannedOperationKind::DeleteFile,
                None,
            )],
        )
        .unwrap();

        assert_eq!(summary.failed.len(), 1);
        assert!(summary.failed[0]
            .error_message
            .contains("escapes sync root"));
    }

    fn planned(
        relative_path: &str,
        kind: PlannedOperationKind,
        size_bytes: Option<i64>,
    ) -> PlannedOperation {
        PlannedOperation {
            relative_path: relative_path.to_owned(),
            kind,
            size_bytes,
        }
    }

    fn scanned_entry(
        relative_path: &str,
        file_kind: &str,
        content_hash: Option<&str>,
        size_bytes: Option<i64>,
    ) -> ScannedEntry {
        ScannedEntry {
            relative_path: relative_path.to_owned(),
            file_kind: file_kind.to_owned(),
            size_bytes,
            modified_at: None,
            content_hash: content_hash.map(str::to_owned),
        }
    }

    fn file_index_entry(
        relative_path: &str,
        file_kind: &str,
        content_hash: Option<&str>,
        size_bytes: Option<i64>,
        deleted: bool,
    ) -> FileIndexEntry {
        FileIndexEntry {
            task_id: "task".to_owned(),
            relative_path: relative_path.to_owned(),
            file_kind: file_kind.to_owned(),
            size_bytes,
            modified_at: None,
            content_hash: content_hash.map(str::to_owned),
            deleted,
            last_seen_at: "1".to_owned(),
            last_synced_at: (!deleted).then(|| "1".to_owned()),
            updated_at: "1".to_owned(),
        }
    }
}
