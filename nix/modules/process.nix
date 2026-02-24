{
  process.manager.implementation = "process-compose";
  processes = {
    mcp.exec = "uv run omni mcp --port 3002";
    agent.exec = "just agent-channel-webhook";
    skill-runner.exec = "uv run omni skill runner start";
  };
}
