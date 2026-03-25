use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use chrono::Utc;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::gateway::studio::search;
use crate::gateway::studio::types::UiProjectConfig;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::types::{SymbolIndexPhase, SymbolIndexStatus};

pub(crate) struct SymbolIndexCoordinator {
    project_root: PathBuf,
    config_root: PathBuf,
    active_fingerprint: Arc<RwLock<Option<String>>>,
    status: Arc<RwLock<SymbolIndexStatus>>,
    spawn_lock: Mutex<()>,
    shutdown_requested: Arc<AtomicBool>,
    build_task: Mutex<Option<JoinHandle<()>>>,
}

impl SymbolIndexCoordinator {
    #[must_use]
    pub(crate) fn new(project_root: PathBuf, config_root: PathBuf) -> Self {
        Self {
            project_root,
            config_root,
            active_fingerprint: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(SymbolIndexStatus::default())),
            spawn_lock: Mutex::new(()),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            build_task: Mutex::new(None),
        }
    }

    pub(crate) fn stop(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
            phase: SymbolIndexPhase::Idle,
            last_error: None,
            updated_at: Some(timestamp_now()),
        };
        if let Some(build_task) = self
            .build_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            build_task.abort();
        }
    }

    pub(crate) fn sync_projects(
        self: &Arc<Self>,
        projects: Vec<UiProjectConfig>,
        index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    ) {
        if projects.is_empty() {
            *index_cache
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
            *self
                .active_fingerprint
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
            *self
                .status
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
                phase: SymbolIndexPhase::Idle,
                last_error: None,
                updated_at: Some(timestamp_now()),
            };
            return;
        }

        let fingerprint = fingerprint_projects(projects.as_slice());
        let current_fingerprint = self
            .active_fingerprint
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let current_status = self.status();
        let current_index = index_cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        if current_fingerprint.as_deref() == Some(fingerprint.as_str())
            && current_index.is_some()
            && matches!(current_status.phase, SymbolIndexPhase::Ready)
        {
            return;
        }

        *index_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        self.maybe_spawn_build(projects, index_cache, fingerprint);
    }

    pub(crate) fn ensure_started(
        self: &Arc<Self>,
        projects: Vec<UiProjectConfig>,
        index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    ) {
        if projects.is_empty() {
            return;
        }

        let fingerprint = fingerprint_projects(projects.as_slice());
        let current_fingerprint = self
            .active_fingerprint
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let current_status = self.status();
        let current_index = index_cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        if current_fingerprint.as_deref() == Some(fingerprint.as_str()) {
            if current_index.is_some() && matches!(current_status.phase, SymbolIndexPhase::Ready) {
                return;
            }
            if matches!(current_status.phase, SymbolIndexPhase::Indexing) {
                return;
            }
        }

        self.maybe_spawn_build(projects, index_cache, fingerprint);
    }

    pub(crate) fn status(&self) -> SymbolIndexStatus {
        self.status
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    #[allow(clippy::too_many_lines)]
    fn maybe_spawn_build(
        self: &Arc<Self>,
        projects: Vec<UiProjectConfig>,
        index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
        fingerprint: String,
    ) {
        let _spawn_guard = self
            .spawn_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let current_fingerprint = self
            .active_fingerprint
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let current_status = self.status();
        let current_index = index_cache
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        if current_fingerprint.as_deref() == Some(fingerprint.as_str()) {
            if current_index.is_some() && matches!(current_status.phase, SymbolIndexPhase::Ready) {
                return;
            }
            if matches!(current_status.phase, SymbolIndexPhase::Indexing) {
                return;
            }
        }

        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(fingerprint.clone());
        self.shutdown_requested.store(false, Ordering::SeqCst);
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
            phase: SymbolIndexPhase::Indexing,
            last_error: None,
            updated_at: Some(timestamp_now()),
        };

        let project_root = self.project_root.clone();
        let config_root = self.config_root.clone();
        let active_fingerprint = Arc::clone(&self.active_fingerprint);
        let status = Arc::clone(&self.status);
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        if let Ok(handle) = Handle::try_current() {
            let build_task = handle.spawn(async move {
                let build = tokio::task::spawn_blocking(move || {
                    search::build_symbol_index(
                        project_root.as_path(),
                        config_root.as_path(),
                        &projects,
                    )
                })
                .await;

                let latest_fingerprint = active_fingerprint
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                if shutdown_requested.load(Ordering::SeqCst)
                    || latest_fingerprint.as_deref() != Some(fingerprint.as_str())
                {
                    return;
                }

                match build {
                    Ok(index) => {
                        *index_cache
                            .write()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            Some(Arc::new(index));
                        *status
                            .write()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            SymbolIndexStatus {
                                phase: SymbolIndexPhase::Ready,
                                last_error: None,
                                updated_at: Some(timestamp_now()),
                            };
                    }
                    Err(error) => {
                        *index_cache
                            .write()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                        *status
                            .write()
                            .unwrap_or_else(std::sync::PoisonError::into_inner) =
                            SymbolIndexStatus {
                                phase: SymbolIndexPhase::Failed,
                                last_error: Some(format!(
                                    "symbol index background task panicked: {error}"
                                )),
                                updated_at: Some(timestamp_now()),
                            };
                    }
                }
            });
            *self
                .build_task
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(build_task);
        } else {
            *self
                .status
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
                phase: SymbolIndexPhase::Failed,
                last_error: Some("Tokio runtime unavailable for symbol index build".to_string()),
                updated_at: Some(timestamp_now()),
            };
        }
    }

    #[cfg(test)]
    pub(crate) fn set_status_for_test(
        &self,
        projects: &[UiProjectConfig],
        status: SymbolIndexStatus,
    ) {
        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(fingerprint_projects(projects));
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = status;
    }

    #[cfg(test)]
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn set_ready_index_for_test(
        &self,
        projects: &[UiProjectConfig],
        index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
        index: UnifiedSymbolIndex,
    ) {
        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(fingerprint_projects(projects));
        *index_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(index));
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
            phase: SymbolIndexPhase::Ready,
            last_error: None,
            updated_at: Some(timestamp_now()),
        };
    }
}

fn fingerprint_projects(projects: &[UiProjectConfig]) -> String {
    projects
        .iter()
        .map(|project| {
            format!(
                "{}|{}|{}",
                project.name,
                project.root,
                project.dirs.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("::")
}

fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_projects_resets_to_idle_when_projects_are_empty() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(Some(Arc::new(UnifiedSymbolIndex::new()))));

        coordinator.sync_projects(Vec::new(), Arc::clone(&index_cache));

        assert!(
            index_cache
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_none()
        );
        assert_eq!(coordinator.status().phase, SymbolIndexPhase::Idle);
    }

    #[tokio::test]
    async fn ensure_started_marks_non_idle_for_configured_projects() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(temp.path().join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            temp.path().join("src").join("lib.rs"),
            "pub struct BackgroundSymbolIndex;\n",
        )
        .unwrap_or_else(|error| panic!("write source: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(None));

        coordinator.ensure_started(
            vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["src".to_string()],
            }],
            Arc::clone(&index_cache),
        );

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert!(matches!(
            coordinator.status().phase,
            SymbolIndexPhase::Indexing | SymbolIndexPhase::Ready
        ));
    }

    #[tokio::test]
    async fn stop_resets_status_to_idle_after_starting_build() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(temp.path().join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            temp.path().join("src").join("lib.rs"),
            "pub struct BackgroundSymbolIndex;\n",
        )
        .unwrap_or_else(|error| panic!("write source: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(None));

        coordinator.ensure_started(
            vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["src".to_string()],
            }],
            Arc::clone(&index_cache),
        );
        coordinator.stop();
        tokio::task::yield_now().await;

        assert_eq!(coordinator.status().phase, SymbolIndexPhase::Idle);
    }
}
