use super::sanitize_reply_for_send;

#[test]
fn sanitize_reply_for_send_strips_think_sections_and_trims_visible_reply() {
    let reply = sanitize_reply_for_send("<think>internal reasoning</think>\n ACK SQLITE \n");
    assert_eq!(reply, "ACK SQLITE");
}

#[test]
fn sanitize_reply_for_send_returns_empty_when_only_think_content_exists() {
    let reply = sanitize_reply_for_send("<think>internal reasoning only</think>");
    assert!(reply.is_empty());
}
