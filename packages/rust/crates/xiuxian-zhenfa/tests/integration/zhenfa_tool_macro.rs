//! End-to-end macro expansion coverage for generated zhenfa tools.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;
use xiuxian_zhenfa::{
    ZhenfaContext, ZhenfaError, ZhenfaOrchestrator, ZhenfaRegistry, ZhenfaTool, schemars,
    serde_json, zhenfa_tool,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct MacroEchoArgs {
    value: String,
}

/// Echo typed payload and append optional suffix from context extensions.
#[zhenfa_tool(name = "macro.echo", description = "Echo payload with optional suffix")]
async fn macro_echo(ctx: &ZhenfaContext, args: MacroEchoArgs) -> Result<String, ZhenfaError> {
    tokio::task::yield_now().await;
    let suffix = ctx
        .get_extension::<String>()
        .map(|value| (*value).clone())
        .unwrap_or_default();
    Ok(format!("{}{}", args.value, suffix))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct MacroCachedArgs {
    value: String,
}

static MACRO_CACHED_CALLS: AtomicUsize = AtomicUsize::new(0);

#[zhenfa_tool(
    name = "macro.cached",
    description = "Echo payload and expose cache-key code path",
    tool_struct = "MacroCachedTool",
    mutation_scope = "macro.cached.write"
)]
async fn macro_cached(_ctx: &ZhenfaContext, args: MacroCachedArgs) -> Result<String, ZhenfaError> {
    tokio::task::yield_now().await;
    let call_no = MACRO_CACHED_CALLS.fetch_add(1, Ordering::SeqCst) + 1;
    Ok(format!("{}#{call_no}", args.value))
}

#[tokio::test]
async fn macro_generated_tool_dispatches_and_builds_schema() {
    let mut registry = ZhenfaRegistry::new();
    registry.register(Arc::new(MacroEchoTool) as Arc<dyn ZhenfaTool>);
    let orchestrator = ZhenfaOrchestrator::new(registry);

    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension("!".to_string());

    let output = orchestrator
        .dispatch("macro.echo", &ctx, serde_json::json!({ "value": "hello" }))
        .await
        .unwrap_or_else(|error| panic!("macro-generated dispatch should succeed: {error}"));
    assert_eq!(output, "hello!");

    let definitions = orchestrator.registry().definitions();
    let definition = definitions
        .get("macro.echo")
        .unwrap_or_else(|| panic!("macro-generated tool definition should be present"));
    assert_eq!(definition["name"], serde_json::json!("macro.echo"));
    assert_eq!(
        definition["parameters"]["type"],
        serde_json::json!("object")
    );
    assert!(definition["parameters"]["properties"]["value"].is_object());
}

#[tokio::test]
async fn macro_generated_tool_maps_invalid_args_to_invalid_arguments_error() {
    let mut registry = ZhenfaRegistry::new();
    registry.register(Arc::new(MacroEchoTool) as Arc<dyn ZhenfaTool>);
    let orchestrator = ZhenfaOrchestrator::new(registry);

    let error = match orchestrator
        .dispatch(
            "macro.echo",
            &ZhenfaContext::default(),
            serde_json::json!({}),
        )
        .await
    {
        Ok(payload) => {
            panic!("missing `value` should fail deserialization, got payload: {payload}")
        }
        Err(error) => error,
    };
    assert!(matches!(error, ZhenfaError::InvalidArguments { .. }));
}

#[test]
fn macro_generated_tool_supports_custom_struct_and_mutation_scope() {
    let tool = MacroCachedTool;

    assert_eq!(tool.id(), "macro.cached");
    assert_eq!(
        tool.mutation_scope(
            &ZhenfaContext::default(),
            &serde_json::json!({ "value": "stable" })
        ),
        Some("macro.cached.write".to_string())
    );
}
