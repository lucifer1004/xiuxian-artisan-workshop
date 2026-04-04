use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{
    LanceBooleanArray, LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray,
    LanceUInt64Array,
};
use xiuxian_wendao_runtime::transport::{VfsScanFlightRouteProvider, VfsScanFlightRouteResponse};

use crate::gateway::studio::router::StudioState;
#[cfg(test)]
use crate::gateway::studio::types::VfsScanEntry;
use crate::gateway::studio::types::{VfsCategory, VfsScanResult};

use super::scan::scan_roots;

/// Studio-backed Flight provider for the semantic `/vfs/scan` route.
#[derive(Clone)]
pub(crate) struct StudioVfsScanFlightRouteProvider {
    studio: Arc<StudioState>,
}

impl StudioVfsScanFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

impl std::fmt::Debug for StudioVfsScanFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioVfsScanFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl VfsScanFlightRouteProvider for StudioVfsScanFlightRouteProvider {
    async fn scan_vfs_batch(&self) -> Result<VfsScanFlightRouteResponse, Status> {
        load_vfs_scan_flight_response(self.studio.as_ref()).map_err(Status::internal)
    }
}

pub(crate) fn load_vfs_scan_flight_response(
    studio: &StudioState,
) -> Result<VfsScanFlightRouteResponse, String> {
    let response = scan_roots(studio);
    let batch = vfs_scan_result_batch(&response)?;
    let app_metadata = vfs_scan_result_flight_app_metadata(&response)?;
    Ok(VfsScanFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
}

pub(crate) fn vfs_scan_result_batch(response: &VfsScanResult) -> Result<LanceRecordBatch, String> {
    let project_dirs_json = response
        .entries
        .iter()
        .map(|entry| {
            entry
                .project_dirs
                .as_ref()
                .map(|project_dirs| encode_project_dirs_json(project_dirs.as_slice()))
                .transpose()
        })
        .collect::<Result<Vec<_>, _>>()?;
    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("name", LanceDataType::Utf8, false),
            LanceField::new("isDir", LanceDataType::Boolean, false),
            LanceField::new("category", LanceDataType::Utf8, false),
            LanceField::new("size", LanceDataType::UInt64, false),
            LanceField::new("modified", LanceDataType::UInt64, false),
            LanceField::new("contentType", LanceDataType::Utf8, true),
            LanceField::new("hasFrontmatter", LanceDataType::Boolean, false),
            LanceField::new("wendaoId", LanceDataType::Utf8, true),
            LanceField::new("projectName", LanceDataType::Utf8, true),
            LanceField::new("rootLabel", LanceDataType::Utf8, true),
            LanceField::new("projectRoot", LanceDataType::Utf8, true),
            LanceField::new("projectDirsJson", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.path.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.name.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceBooleanArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.is_dir)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| vfs_category_as_str(entry.category))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceUInt64Array::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.size)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceUInt64Array::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.modified)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.content_type.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceBooleanArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.has_frontmatter)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.wendao_id.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.project_name.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.root_label.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                response
                    .entries
                    .iter()
                    .map(|entry| entry.project_root.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(
                project_dirs_json
                    .iter()
                    .map(|value: &Option<String>| value.as_deref())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| format!("failed to build VFS scan Flight batch: {error}"))
}

pub(crate) fn vfs_scan_result_flight_app_metadata(
    response: &VfsScanResult,
) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "fileCount": response.file_count,
        "dirCount": response.dir_count,
        "scanDurationMs": response.scan_duration_ms,
    }))
    .map_err(|error| format!("failed to encode VFS scan Flight app metadata: {error}"))
}

fn encode_project_dirs_json(project_dirs: &[String]) -> Result<String, String> {
    serde_json::to_string(project_dirs)
        .map_err(|error| format!("failed to encode VFS scan project dirs: {error}"))
}

fn vfs_category_as_str(category: VfsCategory) -> &'static str {
    match category {
        VfsCategory::Folder => "folder",
        VfsCategory::Skill => "skill",
        VfsCategory::Doc => "doc",
        VfsCategory::Knowledge => "knowledge",
        VfsCategory::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use xiuxian_vector::{LanceArray, LanceStringArray};

    use super::*;

    fn sample_entry() -> VfsScanEntry {
        VfsScanEntry {
            path: "kernel/docs/alpha.md".to_string(),
            name: "alpha.md".to_string(),
            is_dir: false,
            category: VfsCategory::Doc,
            size: 128,
            modified: 42,
            content_type: Some("text/markdown".to_string()),
            has_frontmatter: true,
            wendao_id: Some("doc:alpha".to_string()),
            project_name: Some("kernel".to_string()),
            root_label: Some("docs".to_string()),
            project_root: Some(".".to_string()),
            project_dirs: Some(vec!["docs".to_string()]),
        }
    }

    #[test]
    fn vfs_scan_result_batch_preserves_entry_fields() {
        let batch = vfs_scan_result_batch(&VfsScanResult {
            entries: vec![sample_entry()],
            file_count: 1,
            dir_count: 0,
            scan_duration_ms: 7,
        })
        .expect("build VFS scan batch");

        assert_eq!(batch.num_rows(), 1);
        let project_dirs = batch
            .column_by_name("projectDirsJson")
            .expect("projectDirsJson column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("projectDirsJson column type");
        assert_eq!(project_dirs.value(0), "[\"docs\"]");
    }

    #[test]
    fn vfs_scan_result_metadata_preserves_summary_fields() {
        let metadata = vfs_scan_result_flight_app_metadata(&VfsScanResult {
            entries: vec![sample_entry()],
            file_count: 1,
            dir_count: 0,
            scan_duration_ms: 7,
        })
        .expect("encode VFS scan metadata");

        let payload: serde_json::Value =
            serde_json::from_slice(&metadata).expect("metadata should decode");
        assert_eq!(payload["fileCount"], 1);
        assert_eq!(payload["dirCount"], 0);
        assert_eq!(payload["scanDurationMs"], 7);
    }
}
