{ __inputs__, ... }:
let
  gatewayConfig = "packages/rust/crates/xiuxian-wendao/wendao.toml";
  gatewayTargetDir = ".cache/cargo-target/wendao-gateway-process-compose";
  sentinelTargetDir = ".cache/cargo-target/wendao-sentinel-process-compose";
in
{
  packages = [
    __inputs__.packages.capfox
  ];
  process.manager.implementation = "process-compose";
  processes = {
    valkey = {
      exec = "valkey-server";
      process-compose = {
        readiness_probe = {
          exec.command = "valkey-cli ping";
          initial_delay_seconds = 1;
          period_seconds = 2;
        };
      };
    };

    carfox.exec = "capfox start";
    agent.exec = "just agent-channel-webhook-restart";

    # Wendao Phase 7.6 Integrated Services
    wendao-gateway = {
      exec = "CARGO_TARGET_DIR=${gatewayTargetDir} VALKEY_URL=redis://127.0.0.1:6379/0 cargo run -p xiuxian-wendao -- --conf ${gatewayConfig} gateway start";
      process-compose = {
        depends_on = {
          valkey.condition = "process_healthy";
        };
        readiness_probe = {
          exec.command = ''
            PORT=$(awk -F= '/^[[:space:]]*port[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' ${gatewayConfig})
            if [ -z "$PORT" ]; then
              PORT=9517
            fi
            curl -fsS --max-time 2 "http://127.0.0.1:$PORT/api/health"
          '';
          initial_delay_seconds = 60;
          period_seconds = 5;
          timeout_seconds = 2;
          failure_threshold = 48;
        };
      };
    };

    wendao-sentinel = {
      exec = "CARGO_TARGET_DIR=${sentinelTargetDir} VALKEY_URL=redis://127.0.0.1:6379/0 cargo run -p xiuxian-wendao -- --conf ${gatewayConfig} sentinel watch";
      process-compose = {
        depends_on = {
          wendao-gateway.condition = "process_healthy";
        };
      };
    };
  };
}
