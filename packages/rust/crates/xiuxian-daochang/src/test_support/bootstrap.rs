use super::bindings::resolve_binding_target_from_registry;
use super::model::{
    InternalSkillManifest, InternalSkillNativeAliasCompilation,
    InternalSkillNativeAliasCompileError, InternalSkillNativeAliasSeed,
    InternalSkillNativeAliasSpec, InternalSkillWorkflowType,
};

/// Resolve a validated internal binding id to the concrete native tool name used at runtime.
///
/// # Errors
///
/// Returns an error when the manifest references an unknown internal runtime binding. The error
/// includes the full set of currently supported binding ids for operator-facing diagnostics.
pub fn resolve_internal_skill_binding_target(
    internal_id: &str,
) -> Result<&'static str, InternalSkillNativeAliasCompileError> {
    resolve_binding_target_from_registry(internal_id)
}

/// Compile a validated manifest payload into a runtime-ready native alias spec.
#[must_use]
pub fn compile_internal_skill_native_alias<Workflow>(
    seed: InternalSkillNativeAliasSeed<Workflow>,
) -> Option<InternalSkillNativeAliasSpec<Workflow>> {
    try_compile_internal_skill_native_alias(seed).ok()
}

/// Compile a validated manifest payload into a runtime-ready native alias spec.
///
/// # Errors
///
/// Returns an error when the manifest references an unknown internal runtime binding.
pub fn try_compile_internal_skill_native_alias<Workflow>(
    seed: InternalSkillNativeAliasSeed<Workflow>,
) -> Result<InternalSkillNativeAliasSpec<Workflow>, InternalSkillNativeAliasCompileError> {
    let target_tool_name = resolve_internal_skill_binding_target(seed.internal_id.as_str())?;

    Ok(InternalSkillNativeAliasSpec {
        manifest_id: seed.manifest_id,
        tool_name: seed.tool_name,
        description: seed.description,
        workflow_type: seed.workflow_type,
        internal_id: seed.internal_id,
        metadata: seed.metadata,
        target_tool_name: target_tool_name.to_string(),
        annotations: seed.annotations,
        source_path: seed.source_path,
    })
}

/// Compile a batch of validated internal manifests into native alias specs.
///
/// Manifests with unknown internal bindings are retained in `issues` with source-path context,
/// allowing runtime consumers to stay thin while preserving audit visibility.
#[must_use]
pub fn compile_internal_skill_manifest_aliases(
    manifests: Vec<InternalSkillManifest>,
) -> InternalSkillNativeAliasCompilation<InternalSkillWorkflowType> {
    let mut compilation = InternalSkillNativeAliasCompilation {
        compiled_specs: Vec::with_capacity(manifests.len()),
        issues: Vec::new(),
    };

    for manifest in manifests {
        let source_path = manifest.source_path.clone();
        let seed = InternalSkillNativeAliasSeed {
            manifest_id: manifest.manifest_id,
            tool_name: manifest.tool_name,
            description: manifest.description,
            workflow_type: manifest.workflow_type,
            internal_id: manifest.internal_id,
            metadata: manifest.metadata,
            annotations: manifest.annotations,
            source_path,
        };

        match try_compile_internal_skill_native_alias(seed) {
            Ok(spec) => compilation.compiled_specs.push(spec),
            Err(error) => compilation
                .issues
                .push(format!("{} -> {error}", manifest.source_path.display())),
        }
    }

    compilation
}
