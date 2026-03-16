{ __inputs__, ... }:
{
  packages = [
    __inputs__.packages.capfox
  ];
  process.manager.implementation = "process-compose";
  processes = {
    valkey.exec = "valkey-server";
    mcp.exec = "capfox start";
    agent.exec = "just agent-channel-webhook-restart";
    skill-runner.exec = "uv run omni skill runner start";
  };
}
