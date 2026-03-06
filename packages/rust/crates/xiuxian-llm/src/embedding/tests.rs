use super::sdk::{guard_mistral_call, guard_mistral_future};

#[test]
fn guard_mistral_call_returns_value_on_success() {
    let result = guard_mistral_call("sync_success", || 7usize);
    assert_eq!(result, Some(7));
}

#[test]
fn guard_mistral_call_converts_panic_to_none() {
    let result = guard_mistral_call("sync_panic", || {
        panic!("intentional sync panic for guard test");
    });
    assert_eq!(result, None);
}

#[tokio::test]
async fn guard_mistral_future_returns_value_on_success() {
    let result = guard_mistral_future("unit_success", async { 42usize }).await;
    assert_eq!(result, Some(42));
}

#[tokio::test]
async fn guard_mistral_future_converts_panic_to_none() {
    let result = guard_mistral_future("unit_panic", async {
        panic!("intentional panic for guard test");
    })
    .await;
    assert_eq!(result, None);
}
