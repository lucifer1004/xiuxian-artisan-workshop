use crate::search::repo_entity::schema::RepoEntityRow;
use crate::search::{RepoStagedMutationAction, RepoStagedMutationPlan};

pub(crate) type RepoEntityBuildAction = RepoStagedMutationAction<Vec<RepoEntityRow>>;
pub(crate) type RepoEntityBuildPlan = RepoStagedMutationPlan<Vec<RepoEntityRow>>;
