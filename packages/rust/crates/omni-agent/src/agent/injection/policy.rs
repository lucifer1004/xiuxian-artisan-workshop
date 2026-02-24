use xiuxian_qianhuan::{
    InjectionMode, InjectionOrderStrategy, InjectionPolicy, PromptContextBlock,
};

/// Resolve deterministic injection mode from context shape.
///
/// Policy behavior:
/// - explicit `single` remains `single`
/// - explicit `hybrid` remains `hybrid`
/// - `classified` acts as adaptive default:
///   - `single` for one block
///   - `hybrid` for multi-domain blocks
///   - `classified` otherwise
pub(super) fn resolve_effective_policy(
    mut policy: InjectionPolicy,
    blocks: &[PromptContextBlock],
) -> InjectionPolicy {
    policy.mode = resolve_mode(policy.mode, blocks);
    if matches!(policy.mode, InjectionMode::Single) {
        policy.max_blocks = policy.max_blocks.min(1);
        policy.ordering = InjectionOrderStrategy::PriorityDesc;
    }
    policy
}

fn resolve_mode(configured_mode: InjectionMode, blocks: &[PromptContextBlock]) -> InjectionMode {
    match configured_mode {
        InjectionMode::Single => InjectionMode::Single,
        InjectionMode::Hybrid => InjectionMode::Hybrid,
        InjectionMode::Classified => {
            if blocks.len() <= 1 {
                return InjectionMode::Single;
            }
            if distinct_category_count(blocks) >= 2 {
                return InjectionMode::Hybrid;
            }
            InjectionMode::Classified
        }
    }
}

fn distinct_category_count(blocks: &[PromptContextBlock]) -> usize {
    let mut categories = Vec::new();
    for block in blocks {
        if !categories.contains(&block.category) {
            categories.push(block.category);
        }
    }
    categories.len()
}
