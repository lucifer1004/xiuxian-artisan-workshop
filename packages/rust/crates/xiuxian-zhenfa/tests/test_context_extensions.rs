//! Context extension semantics coverage for native zhenfa dispatch.

use std::sync::Arc;

use xiuxian_zhenfa::ZhenfaContext;

#[test]
fn context_extensions_roundtrip_by_type() {
    let mut ctx = ZhenfaContext::default();
    assert!(!ctx.has_extension::<String>());
    assert_eq!(ctx.extension_count(), 0);

    assert!(
        ctx.insert_extension::<String>("alpha".to_string())
            .is_none()
    );
    assert!(ctx.has_extension::<String>());
    assert_eq!(ctx.extension_count(), 1);

    let stored = ctx
        .get_extension::<String>()
        .unwrap_or_else(|| panic!("string extension should exist"));
    assert_eq!(stored.as_str(), "alpha");

    let previous = ctx
        .insert_extension::<String>("beta".to_string())
        .unwrap_or_else(|| panic!("previous string extension should be returned"));
    assert_eq!(previous.as_str(), "alpha");
    let replaced = ctx
        .get_extension::<String>()
        .unwrap_or_else(|| panic!("replacement extension should exist"));
    assert_eq!(replaced.as_str(), "beta");
}

#[test]
fn context_extensions_clone_uses_copy_on_write_registry() {
    let mut ctx = ZhenfaContext::default();
    assert!(
        ctx.insert_shared_extension::<String>(Arc::new("shared".to_string()))
            .is_none()
    );

    let mut cloned = ctx.clone();
    let value_from_clone = cloned
        .get_extension::<String>()
        .unwrap_or_else(|| panic!("cloned context should read extension"));
    assert_eq!(value_from_clone.as_str(), "shared");

    let _ = cloned.insert_extension::<usize>(7);
    assert!(!ctx.has_extension::<usize>());
    let cloned_seen = cloned
        .get_extension::<usize>()
        .unwrap_or_else(|| panic!("extension inserted into clone should be visible in clone"));
    assert_eq!(*cloned_seen, 7);
}
