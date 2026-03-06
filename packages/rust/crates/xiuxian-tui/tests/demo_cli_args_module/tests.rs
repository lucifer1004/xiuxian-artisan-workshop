use super::*;

#[test]
fn test_demo_args_parsing() {
    let args = DemoArgs::parse_from(["xiuxian-tui-demo", "--socket", "/test.sock"]);
    assert_eq!(args.socket, "/test.sock");
    assert!(!args.demo);
}
