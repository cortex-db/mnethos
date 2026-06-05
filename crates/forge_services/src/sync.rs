use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use forge_app::{FileReaderInfra, SyncProgressCounter, WorkspaceStatus, compute_hash};
use forge_domain::{ApiKey, FileHash, SyncProgress, UserId, WorkspaceId, WorkspaceIndexRepository};
use futures::stream::{Stream, StreamExt};
use tracing::{info, warn};

use crate::fd::{FileDiscovery, discover_sync_file_paths};

/// Error type for a single file that could not be read during workspace
/// operations, carrying the file path for downstream reporting.
#[derive(Debug, thiserror::Error)]
#[error("Failed to read file '{path}': {source}")]
struct FileReadError {
    path: PathBuf,
    #[source]
    source: anyhow::Error,
}

/// Maximum cumulative size, in bytes, of file content packed into a single
/// upload request.
///
/// The server builds an embedding request from each upload and enforces the
/// default 4 MiB (4,194,304-byte) gRPC message-size limit on it. This budget is
/// set below that limit to leave headroom for protobuf framing and the file
/// path strings that accompany each file's content, so a full batch never trips
/// the server-side "decoded message length too large" error.
const MAX_UPLOAD_BATCH_BYTES: usize = 3 * 1024 * 1024;

/// Canonicalizes `path`, attaching a context message that includes the original
/// path on failure.
pub fn canonicalize_path(path: PathBuf) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", path.display()))
}

/// Extracts [`forge_domain::FileStatus`] entries with
/// [`forge_domain::SyncStatus::Failed`] from a slice of file-read results by
/// downcasting errors to [`FileReadError`].
fn extract_failed_statuses<T>(results: &[Result<T>]) -> Vec<forge_domain::FileStatus> {
    results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .filter_map(|e| e.downcast_ref::<FileReadError>())
        .map(|e| {
            forge_domain::FileStatus::new(
                e.path.to_string_lossy().into_owned(),
                forge_domain::SyncStatus::Failed,
            )
        })
        .collect()
}

/// Extracts human-readable failure reasons from a slice of file-read results by
/// downcasting errors to [`FileReadError`].
///
/// Only [`FileReadError`] entries are reported so the reasons stay aligned with
/// the failed-file count derived from [`extract_failed_statuses`].
fn extract_failure_reasons<T>(results: &[Result<T>]) -> Vec<String> {
    results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .filter_map(|e| e.downcast_ref::<FileReadError>())
        .map(|e| e.to_string())
        .collect()
}

/// Removes duplicate failure reasons while preserving first-seen order.
///
/// A single root cause (for example an authentication or network failure)
/// typically surfaces once per upload batch; deduplication keeps the reported
/// error concise without dropping distinct failures.
fn dedup_reasons(reasons: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    reasons
        .into_iter()
        .filter(|reason| seen.insert(reason.clone()))
        .collect()
}

/// Handles all sync operations for a workspace.
///
/// `F` provides infrastructure capabilities (file I/O, workspace index) and
/// `D` is the file-discovery strategy used to enumerate workspace files.
pub struct WorkspaceSyncEngine<F, D> {
    infra: Arc<F>,
    discovery: Arc<D>,
    workspace_root: PathBuf,
    workspace_id: WorkspaceId,
    user_id: UserId,
    token: ApiKey,
    batch_size: usize,
}

impl<F, D> WorkspaceSyncEngine<F, D> {
    /// Creates a new workspace sync engine with the provided infrastructure,
    /// file-discovery strategy, and workspace context shared by all operations.
    pub fn new(
        infra: Arc<F>,
        discovery: Arc<D>,
        workspace_root: PathBuf,
        workspace_id: WorkspaceId,
        user_id: UserId,
        token: ApiKey,
        batch_size: usize,
    ) -> Self {
        Self {
            infra,
            discovery,
            workspace_root,
            workspace_id,
            user_id,
            token,
            batch_size,
        }
    }
}

impl<F: 'static + WorkspaceIndexRepository + FileReaderInfra, D: FileDiscovery + 'static>
    WorkspaceSyncEngine<F, D>
{
    /// Executes the full workspace sync, emitting progress events via `emit`.
    ///
    /// Reads local file hashes, compares them against remote, then deletes
    /// stale files and uploads new or modified ones.
    pub async fn run<E, Fut>(&self, emit: E) -> Result<()>
    where
        E: Fn(SyncProgress) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = ()> + Send,
    {
        emit(SyncProgress::DiscoveringFiles {
            path: self.workspace_root.clone(),
            workspace_id: self.workspace_id.clone(),
        })
        .await;

        // Pass 1: stream files and collect only hashes — content is discarded
        // immediately after hashing so peak memory is bounded to one batch
        // of file content rather than the entire workspace.
        let results: Vec<Result<FileHash>> = self.read_hashes().collect().await;
        let failed_statuses = extract_failed_statuses(&results);
        // Capture the underlying read failures so they can be surfaced to the
        // user if the sync ultimately fails, instead of only being logged.
        let mut failure_reasons = extract_failure_reasons(&results);
        let local_hashes: Vec<FileHash> = results.into_iter().flatten().collect();

        let total_file_count = local_hashes.len() + failed_statuses.len();
        emit(SyncProgress::FilesDiscovered { count: total_file_count }).await;

        let remote_files = self.fetch_remote_hashes().await?;

        emit(SyncProgress::ComparingFiles {
            remote_files: remote_files.len(),
            local_files: total_file_count,
        })
        .await;

        let plan = WorkspaceStatus::new(self.workspace_root.clone(), remote_files);
        let mut statuses = plan.file_statuses(local_hashes.clone());
        statuses.extend(failed_statuses);

        // Compute counts from statuses
        let added = statuses
            .iter()
            .filter(|s| s.status == forge_domain::SyncStatus::New)
            .count();
        let deleted = statuses
            .iter()
            .filter(|s| s.status == forge_domain::SyncStatus::Deleted)
            .count();
        let modified = statuses
            .iter()
            .filter(|s| s.status == forge_domain::SyncStatus::Modified)
            .count();
        let mut failed_files = statuses
            .iter()
            .filter(|s| s.status == forge_domain::SyncStatus::Failed)
            .count();

        // Compute total number of affected files
        let total_file_changes = added + deleted + modified;

        // Only emit diff computed event if there are actual changes
        if total_file_changes > 0 {
            emit(SyncProgress::DiffComputed { added, deleted, modified }).await;
        }

        // Derive the exact paths to delete/upload — no file content required
        let sync_paths = plan.get_sync_paths(local_hashes);

        let total_operations = sync_paths.delete.len() + sync_paths.upload.len();
        let mut counter = SyncProgressCounter::new(total_file_changes, total_operations);

        emit(counter.sync_progress()).await;

        // Delete all files in a single batched call
        match self.delete_files(sync_paths.delete.clone()).await {
            Ok(deleted_count) => {
                counter.complete(deleted_count);
                emit(counter.sync_progress()).await;
            }
            Err(e) => {
                warn!(workspace_id = %self.workspace_id, error = ?e, "Failed to delete files during sync");
                failed_files += sync_paths.delete.len();
                failure_reasons.push(format!("{e:#}"));
            }
        }

        // Pass 2: upload files — files are grouped into batches of `batch_size`
        // and each batch is sent in a single HTTP request, sequentially.
        let mut upload_stream = Box::pin(self.upload_files(sync_paths.upload));

        // Process uploads as they complete, updating progress incrementally
        while let Some((attempted, result)) = upload_stream.next().await {
            match result {
                Ok(()) => {
                    counter.complete(attempted);
                    emit(counter.sync_progress()).await;
                }
                Err(e) => {
                    warn!(workspace_id = %self.workspace_id, error = ?e, "Failed to upload file during sync");
                    failed_files += attempted;
                    failure_reasons.push(format!("{e:#}"));
                    // Continue processing remaining uploads
                }
            }
        }

        info!(
            workspace_id = %self.workspace_id,
            total_files = total_file_count,
            "Sync completed successfully"
        );

        emit(SyncProgress::Completed {
            total_files: total_file_count,
            uploaded_files: total_file_changes,
            failed_files,
        })
        .await;

        // Fail if there were any failed files, surfacing the deduplicated
        // reasons so the user can see why the sync failed.
        if failed_files > 0 {
            Err(
                forge_domain::Error::sync_failed(failed_files, dedup_reasons(failure_reasons))
                    .into(),
            )
        } else {
            Ok(())
        }
    }

    /// Computes the current sync status for all files in the workspace.
    ///
    /// Reads local file hashes and compares them against the remote server to
    /// produce a per-file status report.
    pub async fn compute_status(&self) -> Result<Vec<forge_domain::FileStatus>> {
        let results: Vec<Result<FileHash>> = self.read_hashes().collect().await;

        let mut failed_statuses = extract_failed_statuses(&results);
        let local_hashes: Vec<FileHash> = results.into_iter().flatten().collect();

        let remote_files = self.fetch_remote_hashes().await?;

        let plan = WorkspaceStatus::new(self.workspace_root.clone(), remote_files);
        let mut statuses = plan.file_statuses(local_hashes);
        statuses.append(&mut failed_statuses);
        Ok(statuses)
    }

    /// Fetches remote file hashes from the server.
    async fn fetch_remote_hashes(&self) -> anyhow::Result<Vec<FileHash>> {
        info!(workspace_id = %self.workspace_id, "Fetching existing file hashes from server to detect changes...");
        let workspace_files =
            forge_domain::CodeBase::new(self.user_id.clone(), self.workspace_id.clone(), ());
        self.infra
            .list_workspace_files(&workspace_files, &self.token)
            .await
    }

    /// Deletes files from the workspace.
    ///
    /// Returns the number of files that were successfully deleted.
    async fn delete_files(&self, files_to_delete: Vec<PathBuf>) -> Result<usize> {
        if files_to_delete.is_empty() {
            return Ok(0);
        }

        let paths: Vec<String> = files_to_delete
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        let deletion =
            forge_domain::CodeBase::new(self.user_id.clone(), self.workspace_id.clone(), paths);
        self.infra
            .delete_files(&deletion, &self.token)
            .await
            .context("Failed to delete files")?;

        for path in &files_to_delete {
            info!(workspace_id = %self.workspace_id, path = %path.display(), "File deleted successfully");
        }

        Ok(files_to_delete.len())
    }

    /// Uploads a single batch of already-read files in one request.
    ///
    /// The files are wrapped in a [`forge_domain::FileUpload`] payload and sent
    /// to the workspace index. Any transport or server error is surfaced with a
    /// `"Failed to upload files"` context.
    async fn upload_batch(
        infra: &F,
        user_id: &UserId,
        workspace_id: &WorkspaceId,
        token: &ApiKey,
        files: Vec<forge_domain::FileRead>,
    ) -> Result<()> {
        let upload = forge_domain::CodeBase::new(user_id.clone(), workspace_id.clone(), files);
        infra
            .upload_files(&upload, token)
            .await
            .context("Failed to upload files")?;
        Ok(())
    }

    /// Uploads files in size-bounded batches, sending one HTTP request per
    /// batch.
    ///
    /// Files are read from disk and accumulated into a batch until adding the
    /// next file would push the batch past [`MAX_UPLOAD_BATCH_BYTES`] or the
    /// batch reaches `batch_size` files, at which point the batch is uploaded
    /// in a single request. Sizing batches by their cumulative byte budget
    /// — rather than file count alone — keeps every request safely under
    /// the server's gRPC message-size limit even when files are large; a
    /// file that exceeds the budget on its own is uploaded alone as a
    /// best-effort attempt.
    ///
    /// Batches are processed sequentially — only one HTTP request is in-flight
    /// at a time — which bounds both memory usage and server concurrency. The
    /// stream yields the number of files attempted per batch along with whether
    /// the batch upload succeeded. A file that cannot be read is reported as a
    /// single-file failure without aborting the batch already accumulated.
    fn upload_files(
        &self,
        paths: Vec<PathBuf>,
    ) -> impl Stream<Item = (usize, Result<(), anyhow::Error>)> + Send {
        let user_id = self.user_id.clone();
        let workspace_id = self.workspace_id.clone();
        let token = self.token.clone();
        let infra = self.infra.clone();
        let batch_size = self.batch_size;

        async_stream::stream! {
            let mut batch: Vec<forge_domain::FileRead> = Vec::new();
            let mut batch_bytes: usize = 0;

            for file_path in paths {
                let content = match infra.read_utf8(&file_path).await.with_context(|| {
                    format!("Failed to read file '{}' for upload", file_path.display())
                }) {
                    Ok(content) => content,
                    Err(e) => {
                        // Isolate an unreadable file to its own failure so the
                        // batch already accumulated is still uploaded.
                        yield (1, Err(e));
                        continue;
                    }
                };

                let file_bytes = content.len();

                // Flush the current batch before adding a file that would push
                // it past the byte budget or the count limit. A non-empty guard
                // ensures a file larger than the budget is still attempted on
                // its own rather than being dropped.
                if !batch.is_empty()
                    && (batch_bytes + file_bytes > MAX_UPLOAD_BATCH_BYTES
                        || batch.len() >= batch_size)
                {
                    let attempted = batch.len();
                    let result = Self::upload_batch(
                        infra.as_ref(),
                        &user_id,
                        &workspace_id,
                        &token,
                        std::mem::take(&mut batch),
                    )
                    .await;
                    batch_bytes = 0;
                    yield (attempted, result);
                }

                batch.push(forge_domain::FileRead::new(
                    file_path.to_string_lossy().into_owned(),
                    content,
                ));
                batch_bytes += file_bytes;
            }

            // Flush any trailing batch.
            if !batch.is_empty() {
                let attempted = batch.len();
                let result = Self::upload_batch(
                    infra.as_ref(),
                    &user_id,
                    &workspace_id,
                    &token,
                    batch,
                )
                .await;
                yield (attempted, result);
            }
        }
    }

    /// Discovers workspace files and streams their hashes without retaining
    /// file content in memory.
    ///
    /// Each file is read in batches for concurrency, but the content is
    /// discarded immediately after the hash is computed so that only one
    /// batch of file content occupies memory at a time.
    fn read_hashes(&self) -> impl Stream<Item = Result<FileHash>> + Send {
        let dir_path = self.workspace_root.clone();
        let infra = self.infra.clone();
        let discovery = self.discovery.clone();
        let workspace_id = self.workspace_id.clone();
        let batch_size = self.batch_size;

        async_stream::stream! {
            let file_paths: Vec<PathBuf> = match discover_sync_file_paths(
                discovery.as_ref(),
                &dir_path,
                &workspace_id,
            ).await {
                Ok(file_paths) => file_paths,
                Err(err) => {
                    yield Err(err);
                    return;
                }
            };

            let stream = infra.read_batch_utf8(batch_size, file_paths);
            futures::pin_mut!(stream);

            while let Some((absolute_path, result)) = stream.next().await {
                match result {
                    Ok(content) => {
                        let hash = compute_hash(&content);
                        // content is dropped here — only the hash is retained
                        let path_str = absolute_path.to_string_lossy().to_string();
                        yield Ok(FileHash { path: path_str, hash });
                    }
                    Err(e) => {
                        warn!(path = %absolute_path.display(), error = ?e, "Skipping unreadable file during sync");
                        yield Err(FileReadError { path: absolute_path, source: e }.into());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use async_trait::async_trait;
    use forge_app::FileReaderInfra;
    use forge_domain::{
        ApiKey, FileDeletion, FileHash, FileInfo, FileUpload, FileUploadInfo, Node, UserId,
        WorkspaceAuth, WorkspaceFiles, WorkspaceId, WorkspaceIndexRepository, WorkspaceInfo,
    };
    use futures::StreamExt;
    use pretty_assertions::assert_eq;

    use super::*;

    /// Minimal infra for `upload_files`: only `read_utf8` + `upload_files`
    /// are exercised; everything else is intentionally unreachable.
    ///
    /// `read_utf8` derives content size from the file name so byte-budget
    /// batching can be tested deterministically: a numeric name yields that
    /// many bytes, the name `ERR` fails the read, and any other name yields
    /// empty content.
    struct MockInfra {
        fail_upload: bool,
    }

    #[async_trait]
    #[rustfmt::skip]
    impl FileReaderInfra for MockInfra {
        async fn read_utf8(&self, path: &Path) -> anyhow::Result<String> {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name == "ERR" { return Err(anyhow::anyhow!("read failed")); }
            Ok("a".repeat(name.parse::<usize>().unwrap_or(0)))
        }
        fn read_batch_utf8(&self, _: usize, _: Vec<PathBuf>) -> impl futures::Stream<Item = (PathBuf, anyhow::Result<String>)> + Send { futures::stream::empty() }
        async fn read(&self, _: &Path) -> anyhow::Result<Vec<u8>> { unreachable!() }
        async fn range_read_utf8(&self, _: &Path, _: u64, _: u64) -> anyhow::Result<(String, FileInfo)> { unreachable!() }
    }

    #[async_trait]
    #[rustfmt::skip]
    impl WorkspaceIndexRepository for MockInfra {
        async fn upload_files(&self, _: &FileUpload, _: &ApiKey) -> anyhow::Result<FileUploadInfo> {
            if self.fail_upload { Err(anyhow::anyhow!("boom")) } else { Ok(FileUploadInfo::default()) }
        }
        async fn authenticate(&self) -> anyhow::Result<WorkspaceAuth> { unreachable!() }
        async fn create_workspace(&self, _: &Path, _: &ApiKey) -> anyhow::Result<WorkspaceId> { unreachable!() }
        async fn search(&self, _: &forge_domain::CodeSearchQuery<'_>, _: &ApiKey) -> anyhow::Result<Vec<Node>> { unreachable!() }
        async fn list_workspaces(&self, _: &ApiKey) -> anyhow::Result<Vec<WorkspaceInfo>> { unreachable!() }
        async fn get_workspace(&self, _: &WorkspaceId, _: &ApiKey) -> anyhow::Result<Option<WorkspaceInfo>> { unreachable!() }
        async fn list_workspace_files(&self, _: &WorkspaceFiles, _: &ApiKey) -> anyhow::Result<Vec<FileHash>> { unreachable!() }
        async fn delete_files(&self, _: &FileDeletion, _: &ApiKey) -> anyhow::Result<()> { unreachable!() }
        async fn delete_workspace(&self, _: &WorkspaceId, _: &ApiKey) -> anyhow::Result<()> { unreachable!() }
    }

    /// Discovery is not invoked by `upload_files`; a no-op satisfies the bound.
    struct NoDiscovery;
    #[async_trait]
    impl FileDiscovery for NoDiscovery {
        async fn discover(&self, _: &Path) -> anyhow::Result<Vec<PathBuf>> {
            Ok(vec![])
        }
    }

    fn fixture(
        fail_upload: bool,
        batch_size: usize,
    ) -> WorkspaceSyncEngine<MockInfra, NoDiscovery> {
        WorkspaceSyncEngine::new(
            Arc::new(MockInfra { fail_upload }),
            Arc::new(NoDiscovery),
            PathBuf::new(),
            WorkspaceId::generate(),
            UserId::generate(),
            ApiKey::from(String::new()),
            batch_size,
        )
    }

    /// Regression test for the bug where a failed batch counted as 1 failure
    /// instead of N. With batch_size=3 and 5 files, batches are [3, 2] and
    /// each must report its full size on failure.
    #[tokio::test]
    async fn test_failed_batch_reports_full_batch_size() {
        let engine = fixture(true, 3);
        let paths: Vec<PathBuf> = (0..5).map(|i| PathBuf::from(format!("f{i}"))).collect();

        let actual: Vec<(usize, bool)> = engine
            .upload_files(paths)
            .map(|(n, r)| (n, r.is_ok()))
            .collect()
            .await;
        let expected = vec![(3, false), (2, false)];

        assert_eq!(actual, expected);
    }

    /// Successful batches must also yield their full size so the progress
    /// counter advances correctly.
    #[tokio::test]
    async fn test_successful_batch_reports_full_batch_size() {
        let engine = fixture(false, 3);
        let paths: Vec<PathBuf> = (0..5).map(|i| PathBuf::from(format!("f{i}"))).collect();

        let actual: Vec<(usize, bool)> = engine
            .upload_files(paths)
            .map(|(n, r)| (n, r.is_ok()))
            .collect()
            .await;
        let expected = vec![(3, true), (2, true)];

        assert_eq!(actual, expected);
    }

    /// Batches are split by their cumulative byte budget, independent of the
    /// file-count limit. With four files each sized at a third of the budget
    /// and a high count limit, the first three fill the budget and the
    /// fourth spills into a second batch.
    #[tokio::test]
    async fn test_upload_batches_split_by_byte_budget() {
        // High count limit so only the byte budget drives the split.
        let engine = fixture(false, 100);
        let third = MAX_UPLOAD_BATCH_BYTES / 3;
        let paths: Vec<PathBuf> = (0..4).map(|_| PathBuf::from(third.to_string())).collect();

        let actual: Vec<(usize, bool)> = engine
            .upload_files(paths)
            .map(|(n, r)| (n, r.is_ok()))
            .collect()
            .await;
        let expected = vec![(3, true), (1, true)];

        assert_eq!(actual, expected);
    }

    /// A file larger than the byte budget is uploaded alone rather than
    /// dropped, keeping surrounding small files in their own batches.
    #[tokio::test]
    async fn test_oversized_file_uploaded_alone() {
        let engine = fixture(false, 100);
        let big = MAX_UPLOAD_BATCH_BYTES + 1;
        let paths = vec![
            PathBuf::from("1"),
            PathBuf::from(big.to_string()),
            PathBuf::from("1"),
        ];

        let actual: Vec<(usize, bool)> = engine
            .upload_files(paths)
            .map(|(n, r)| (n, r.is_ok()))
            .collect()
            .await;
        let expected = vec![(1, true), (1, true), (1, true)];

        assert_eq!(actual, expected);
    }

    /// A file that cannot be read fails on its own without aborting the batch
    /// of readable files accumulated around it.
    #[tokio::test]
    async fn test_unreadable_file_isolated_from_batch() {
        let engine = fixture(false, 100);
        let paths = vec![PathBuf::from("1"), PathBuf::from("ERR"), PathBuf::from("1")];

        let actual: Vec<(usize, bool)> = engine
            .upload_files(paths)
            .map(|(n, r)| (n, r.is_ok()))
            .collect()
            .await;
        // The unreadable file yields a single-file failure mid-stream; the two
        // readable files flush together as the trailing batch.
        let expected = vec![(1, false), (2, true)];

        assert_eq!(actual, expected);
    }

    /// Only [`FileReadError`] reasons are extracted; unrelated errors are
    /// ignored so the reasons stay aligned with the failed-file count.
    #[test]
    fn test_extract_failure_reasons_only_file_read_errors() {
        let fixture: Vec<Result<()>> = vec![
            Ok(()),
            Err(FileReadError {
                path: PathBuf::from("logo.png"),
                source: anyhow::anyhow!("stream did not contain valid UTF-8"),
            }
            .into()),
            Err(anyhow::anyhow!("unrelated error")),
        ];

        let actual = extract_failure_reasons(&fixture);
        let expected =
            vec!["Failed to read file 'logo.png': stream did not contain valid UTF-8".to_string()];

        assert_eq!(actual, expected);
    }

    /// Duplicate reasons collapse to a single entry while preserving the order
    /// in which each distinct reason first appeared.
    #[test]
    fn test_dedup_reasons_preserves_first_seen_order() {
        let fixture = vec![
            "upload failed".to_string(),
            "read failed".to_string(),
            "upload failed".to_string(),
            "read failed".to_string(),
            "delete failed".to_string(),
        ];

        let actual = dedup_reasons(fixture);
        let expected = vec![
            "upload failed".to_string(),
            "read failed".to_string(),
            "delete failed".to_string(),
        ];

        assert_eq!(actual, expected);
    }
}
