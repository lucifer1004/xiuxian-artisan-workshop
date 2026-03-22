//! Virtual File System (VFS) orchestration for Studio API.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use walkdir::{DirEntry, WalkDir};

use crate::analyzers::config::RegisteredRepository;
use crate::gateway::studio::pathing::resolve_path_like;
use crate::gateway::studio::router::{StudioState, configured_repositories};
use crate::gateway::studio::types::{
    StudioNavigationTarget, VfsCategory, VfsContentResponse, VfsEntry, VfsScanEntry, VfsScanResult,
};
use crate::git::checkout::{RepositorySyncMode, resolve_repository_source};

pub type VfsError = crate::gateway::studio::router::StudioApiError;

#[derive(Debug, Clone)]
pub struct ProjectFileFilter {
    pub root: PathBuf,
    pub allowed_subdirs: HashSet<PathBuf>,
}

impl ProjectFileFilter {
    pub fn matches(&self, path: &Path) -> bool {
        if !path.starts_with(&self.root) {
            return false;
        }
        if self.allowed_subdirs.is_empty() {
            return true;
        }
        self.allowed_subdirs
            .iter()
            .any(|subdir| path.starts_with(subdir))
    }
}

pub(crate) struct VfsRoot {
    pub request_root: String,
    pub full_path: PathBuf,
    pub project_name: Option<String>,
    pub root_label: Option<String>,
    pub filter_prefix: String,
    pub file_filters: Vec<ProjectFileFilter>,
}

struct VfsCounters {
    files: usize,
    dirs: usize,
}

pub(crate) fn scan_all_roots(state: &StudioState) -> VfsScanResult {
    let start = Instant::now();
    let mut entries = Vec::new();
    let mut counters = VfsCounters { files: 0, dirs: 0 };

    let roots = resolve_all_vfs_roots(state);
    let config = state.ui_config();

    for root in roots {
        scan_directory(
            &root.full_path,
            root.project_name.as_deref(),
            root.root_label.as_deref(),
            root.request_root.as_str(),
            root.filter_prefix.as_str(),
            &config,
            &mut counters,
            &root.file_filters,
            &mut entries,
        );
    }

    VfsScanResult {
        entries,
        file_count: counters.files,
        dir_count: counters.dirs,
        scan_duration_ms: elapsed_millis_u64(start),
    }
}

pub(crate) fn scan_roots(state: &StudioState) -> VfsScanResult {
    if let Some(existing) = state
        .vfs_scan
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
    {
        return existing.clone();
    }

    let result = scan_all_roots(state);
    let mut guard = state
        .vfs_scan
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(existing) = guard.as_ref() {
        return existing.clone();
    }
    *guard = Some(result.clone());
    result
}

pub(crate) fn list_root_entries(state: &StudioState) -> Vec<VfsEntry> {
    let mut entries = Vec::new();

    for root in resolve_all_vfs_roots(state) {
        let metadata = fs::metadata(root.full_path.as_path()).ok();
        let modified = metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());
        let project_dirs = root.file_filters.first().map(|filter| {
            filter
                .allowed_subdirs
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        });
        entries.push(VfsEntry {
            path: root.request_root.clone(),
            name: root
                .root_label
                .clone()
                .or_else(|| root.project_name.clone())
                .unwrap_or_else(|| root.request_root.clone()),
            is_dir: metadata.as_ref().is_none_or(fs::Metadata::is_dir),
            size: metadata.as_ref().map_or(0, fs::Metadata::len),
            modified,
            content_type: None,
            project_name: root.project_name.clone(),
            root_label: root.root_label.clone(),
            project_root: Some(root.full_path.to_string_lossy().to_string()),
            project_dirs,
        });
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

pub(crate) fn resolve_navigation_target(state: &StudioState, path: &str) -> StudioNavigationTarget {
    let normalized = path.trim().trim_start_matches('/').to_string();
    let project_name = state
        .configured_projects()
        .into_iter()
        .find(|project| {
            normalized == project.name
                || normalized.starts_with(format!("{}/", project.name).as_str())
        })
        .map(|project| project.name);

    StudioNavigationTarget {
        path: normalized,
        category: "file".to_string(),
        project_name,
        root_label: None,
        line: None,
        line_end: None,
        column: None,
    }
}

fn resolve_all_vfs_roots(state: &StudioState) -> Vec<VfsRoot> {
    let mut roots = Vec::new();
    let projects = state.configured_projects();

    for project in projects {
        let project_name = Some(project.name.clone());
        let Some(project_root) = resolve_path_like(&state.config_root, project.root.as_str())
        else {
            continue;
        };

        let file_filters = compile_project_filters(&project_root, &project.dirs);

        roots.push(VfsRoot {
            request_root: project.name.clone(),
            full_path: project_root,
            project_name,
            root_label: None,
            filter_prefix: String::new(),
            file_filters,
        });
    }

    let repositories = configured_repositories(state);
    for repository in repositories {
        let Some(root) = resolve_repo_vfs_root(state, &repository) else {
            continue;
        };
        roots.push(root);
    }

    roots
}

fn resolve_repo_vfs_root(
    state: &StudioState,
    repository: &RegisteredRepository,
) -> Option<VfsRoot> {
    let source = resolve_repository_source(
        repository,
        state.config_root.as_path(),
        RepositorySyncMode::Status,
    )
    .ok()?;

    if !source.checkout_root.is_dir() {
        return None;
    }

    let checkout_root = source.checkout_root;
    Some(VfsRoot {
        request_root: repository.id.clone(),
        full_path: checkout_root.clone(),
        project_name: Some(repository.id.clone()),
        root_label: None,
        filter_prefix: String::new(),
        file_filters: vec![ProjectFileFilter {
            root: checkout_root,
            allowed_subdirs: HashSet::new(),
        }],
    })
}

fn compile_project_filters(root: &Path, dirs: &[String]) -> Vec<ProjectFileFilter> {
    let mut allowed_subdirs = HashSet::new();
    for dir in dirs {
        allowed_subdirs.insert(root.join(dir));
    }
    vec![ProjectFileFilter {
        root: root.to_path_buf(),
        allowed_subdirs,
    }]
}

fn elapsed_millis_u64(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn unix_timestamp_secs(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs())
}

#[allow(clippy::too_many_arguments)]
fn scan_directory(
    dir_path: &Path,
    project_name: Option<&str>,
    root_label: Option<&str>,
    request_root: &str,
    filter_prefix: &str,
    _config: &crate::gateway::studio::types::UiConfig,
    counters: &mut VfsCounters,
    filters: &[ProjectFileFilter],
    entries: &mut Vec<VfsScanEntry>,
) {
    let walk = WalkDir::new(dir_path)
        .max_depth(10)
        .into_iter()
        .filter_entry(|e| !should_skip_entry(e));

    for entry in walk.flatten() {
        let path = entry.path();
        if !filters.iter().any(|f| f.matches(path)) {
            continue;
        }

        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map_or(0, fs::Metadata::len);
        let modified = metadata
            .as_ref()
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());

        let is_dir = entry.file_type().is_dir();
        if is_dir {
            counters.dirs += 1;
        } else {
            counters.files += 1;
        }

        let rel_path = path.strip_prefix(dir_path).unwrap_or(path);
        let display_path = if filter_prefix.is_empty() {
            format!("{}/{}", request_root, rel_path.display())
        } else {
            format!("{}/{}/{}", request_root, filter_prefix, rel_path.display())
        };

        entries.push(VfsScanEntry {
            path: display_path.replace('\\', "/"),
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir,
            category: guess_category(entry.path()),
            size,
            modified,
            content_type: None,
            has_frontmatter: false,
            wendao_id: None,
            project_name: project_name.map(String::from),
            root_label: root_label.map(String::from),
            project_root: None,
            project_dirs: None,
        });
    }
}

fn should_skip_entry(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || name == "target" || name == "node_modules"
}

fn guess_category(path: &Path) -> VfsCategory {
    if path.is_dir() {
        return VfsCategory::Folder;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("md" | "markdown") => VfsCategory::Doc,
        Some("skill") => VfsCategory::Skill,
        _ => VfsCategory::Other,
    }
}

pub(crate) fn get_entry(state: &StudioState, path: &str) -> Result<VfsEntry, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let metadata = fs::metadata(&resolved.full_path)
        .map_err(|e| VfsError::internal("IO_ERROR", e.to_string(), None))?;

    Ok(VfsEntry {
        path: path.to_string(),
        name: resolved
            .full_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified: unix_timestamp_secs(&metadata),
        content_type: None,
        project_name: None,
        root_label: None,
        project_root: None,
        project_dirs: None,
    })
}

#[allow(clippy::unused_async)]
pub(crate) async fn read_content(
    state: &StudioState,
    path: &str,
) -> Result<VfsContentResponse, VfsError> {
    let resolved = resolve_vfs_path(state, path)?;
    let content = fs::read_to_string(&resolved.full_path)
        .map_err(|e| VfsError::internal("IO_ERROR", e.to_string(), None))?;
    let metadata = fs::metadata(&resolved.full_path)
        .map_err(|e| VfsError::internal("IO_ERROR", e.to_string(), None))?;

    Ok(VfsContentResponse {
        path: path.to_string(),
        content_type: "text/plain".to_string(),
        content,
        modified: unix_timestamp_secs(&metadata),
    })
}

struct ResolvedVfsPath {
    full_path: PathBuf,
}

fn resolve_vfs_path(state: &StudioState, path: &str) -> Result<ResolvedVfsPath, VfsError> {
    for root in resolve_all_vfs_roots(state) {
        if path == root.request_root {
            return Ok(ResolvedVfsPath {
                full_path: root.full_path,
            });
        }
        let prefix = format!("{}/", root.request_root);
        if path.starts_with(&prefix) {
            let rel = &path[prefix.len()..];
            return Ok(ResolvedVfsPath {
                full_path: root.full_path.join(rel),
            });
        }
    }
    Err(VfsError::not_found(format!("VFS path not found: {path}")))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use git2::{Repository, Signature};
    use uuid::Uuid;

    use super::{resolve_all_vfs_roots, resolve_vfs_path, scan_all_roots, scan_roots};
    use crate::gateway::studio::router::{StudioState, configured_repositories};
    use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};
    use crate::git::checkout::{RepositorySyncMode, resolve_repository_source};

    fn init_git_repository(root: &Path) {
        let repository =
            Repository::init(root).unwrap_or_else(|error| panic!("init repository: {error}"));
        fs::write(
            root.join("Project.toml"),
            "name = \"BaseModelica\"\nversion = \"0.1.0\"\n",
        )
        .unwrap_or_else(|error| panic!("write project file: {error}"));
        fs::create_dir_all(root.join("src"))
            .unwrap_or_else(|error| panic!("create src dir: {error}"));
        fs::write(
            root.join("src").join("BaseModelica.jl"),
            "module BaseModelica\nend\n",
        )
        .unwrap_or_else(|error| panic!("write julia source: {error}"));

        let mut index = repository
            .index()
            .unwrap_or_else(|error| panic!("open index: {error}"));
        index
            .add_path(Path::new("Project.toml"))
            .unwrap_or_else(|error| panic!("stage project file: {error}"));
        index
            .add_path(Path::new("src/BaseModelica.jl"))
            .unwrap_or_else(|error| panic!("stage source file: {error}"));
        let tree_id = index
            .write_tree()
            .unwrap_or_else(|error| panic!("write tree: {error}"));
        let tree = repository
            .find_tree(tree_id)
            .unwrap_or_else(|error| panic!("find tree: {error}"));
        let signature = Signature::now("vfs-test", "vfs-test@example.com")
            .unwrap_or_else(|error| panic!("signature: {error}"));
        repository
            .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
            .unwrap_or_else(|error| panic!("commit: {error}"));
    }

    #[test]
    fn scan_all_roots_includes_repo_project_checkout_entries() {
        let source = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        init_git_repository(source.path());
        let repo_id = format!("repo-vfs-{}", Uuid::new_v4());
        let state = StudioState::new();
        state.set_ui_config(UiConfig {
            projects: Vec::new(),
            repo_projects: vec![UiRepoProjectConfig {
                id: repo_id.clone(),
                root: None,
                url: Some(source.path().display().to_string()),
                git_ref: None,
                refresh: Some("manual".to_string()),
                plugins: vec!["julia".to_string()],
            }],
        });
        let repositories = configured_repositories(&state);
        let repository = repositories
            .first()
            .unwrap_or_else(|| panic!("configured repository"));
        resolve_repository_source(
            repository,
            state.config_root.as_path(),
            RepositorySyncMode::Ensure,
        )
        .unwrap_or_else(|error| panic!("materialize checkout before scan: {error}"));

        let result = scan_all_roots(&state);

        assert!(
            result
                .entries
                .iter()
                .any(|entry| entry.path == format!("{repo_id}/src/BaseModelica.jl"))
        );

        for root in resolve_all_vfs_roots(&state) {
            if root.request_root == repo_id && root.full_path.exists() {
                fs::remove_dir_all(root.full_path)
                    .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
            }
        }
    }

    #[test]
    fn resolve_vfs_path_supports_repo_project_checkout_files() {
        let source = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        init_git_repository(source.path());
        let repo_id = format!("repo-vfs-resolve-{}", Uuid::new_v4());
        let state = StudioState::new();
        state.set_ui_config(UiConfig {
            projects: Vec::new(),
            repo_projects: vec![UiRepoProjectConfig {
                id: repo_id.clone(),
                root: None,
                url: Some(source.path().display().to_string()),
                git_ref: None,
                refresh: Some("manual".to_string()),
                plugins: vec!["julia".to_string()],
            }],
        });
        let repositories = configured_repositories(&state);
        let repository = repositories
            .first()
            .unwrap_or_else(|| panic!("configured repository"));
        resolve_repository_source(
            repository,
            state.config_root.as_path(),
            RepositorySyncMode::Ensure,
        )
        .unwrap_or_else(|error| panic!("materialize checkout before resolving: {error}"));

        let resolved = resolve_vfs_path(&state, format!("{repo_id}/src/BaseModelica.jl").as_str())
            .unwrap_or_else(|error| panic!("resolve repo vfs path: {error:?}"));

        assert!(resolved.full_path.is_file());

        for root in resolve_all_vfs_roots(&state) {
            if root.request_root == repo_id && root.full_path.exists() {
                fs::remove_dir_all(root.full_path)
                    .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
            }
        }
    }

    #[test]
    fn scan_roots_reuses_cached_entries_until_ui_config_changes() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let project_root = temp.path().join("workspace");
        let docs_dir = project_root.join("docs");
        fs::create_dir_all(&docs_dir).unwrap_or_else(|error| panic!("create docs dir: {error}"));
        fs::write(docs_dir.join("guide.md"), "# guide\n")
            .unwrap_or_else(|error| panic!("write guide: {error}"));

        let state = StudioState::new();
        state.set_ui_config(UiConfig {
            projects: vec![crate::gateway::studio::types::UiProjectConfig {
                name: "kernel".to_string(),
                root: project_root.display().to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let first = scan_roots(&state);
        assert!(
            first
                .entries
                .iter()
                .any(|entry| entry.path == "kernel/docs/guide.md")
        );
        fs::remove_file(docs_dir.join("guide.md"))
            .unwrap_or_else(|error| panic!("remove guide: {error}"));
        let cached = scan_roots(&state);
        assert_eq!(cached.entries, first.entries);

        let notes_dir = project_root.join("notes");
        fs::create_dir_all(&notes_dir).unwrap_or_else(|error| panic!("create notes dir: {error}"));
        fs::write(notes_dir.join("todo.md"), "# todo\n")
            .unwrap_or_else(|error| panic!("write note: {error}"));

        state.set_ui_config(UiConfig {
            projects: vec![crate::gateway::studio::types::UiProjectConfig {
                name: "kernel".to_string(),
                root: project_root.display().to_string(),
                dirs: vec!["docs".to_string(), "notes".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let refreshed = scan_roots(&state);
        assert!(
            refreshed
                .entries
                .iter()
                .any(|entry| entry.path == "kernel/notes/todo.md")
        );
        assert!(
            refreshed
                .entries
                .iter()
                .all(|entry| entry.path != "kernel/docs/guide.md")
        );
    }
}
