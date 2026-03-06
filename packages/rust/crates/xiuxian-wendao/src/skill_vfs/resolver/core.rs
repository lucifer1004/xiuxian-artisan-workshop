use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use include_dir::Dir;

use super::super::{SkillNamespaceIndex, SkillVfsError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::skill_vfs::resolver) struct EmbeddedSemanticMount {
    pub(in crate::skill_vfs::resolver) crate_id: String,
    pub(in crate::skill_vfs::resolver) references_dir: PathBuf,
}

/// Semantic resource resolver for `wendao://skills/.../references/...`.
#[derive(Debug, Clone, Default)]
pub struct SkillVfsResolver {
    pub(in crate::skill_vfs::resolver) index: SkillNamespaceIndex,
    pub(in crate::skill_vfs::resolver) mounts: HashMap<String, &'static Dir<'static>>,
    pub(in crate::skill_vfs::resolver) embedded_mounts_by_semantic:
        HashMap<String, Vec<EmbeddedSemanticMount>>,
    pub(in crate::skill_vfs::resolver) content_cache: Arc<DashMap<String, Arc<str>>>,
}

impl SkillVfsResolver {
    /// Build resolver by scanning one or more skill roots.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots(roots: &[PathBuf]) -> Result<Self, SkillVfsError> {
        Ok(Self {
            index: SkillNamespaceIndex::build_from_roots(roots)?,
            mounts: HashMap::new(),
            embedded_mounts_by_semantic: HashMap::new(),
            content_cache: Arc::new(DashMap::new()),
        })
    }

    /// Build resolver by scanning roots and enabling embedded resource mount.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots_with_embedded(roots: &[PathBuf]) -> Result<Self, SkillVfsError> {
        Self::from_roots(roots).map(Self::mount_embedded_dir)
    }

    /// Access the underlying semantic namespace index.
    #[must_use]
    pub fn index(&self) -> &SkillNamespaceIndex {
        &self.index
    }
}
