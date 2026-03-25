{ __inputs__, ... }:
let
  gatewayConfig = "packages/rust/crates/xiuxian-wendao/wendao.toml";
  gatewayTargetDir = ".cache/cargo-target/wendao-gateway-process-compose";
  gatewayRuntimeDir = ".run/wendao-gateway";
  gatewayPidFile = "${gatewayRuntimeDir}/wendao.pid";
  sentinelTargetDir = ".cache/cargo-target/wendao-sentinel-process-compose";
  valkeyDataDir = ".data/valkey";
  valkeyPidFile = ".run/valkey/valkey.pid";
  valkeyRuntimeDir = ".run/valkey";
  valkeyUrl = "redis://127.0.0.1:6379/0";
in
{
  packages = [
    __inputs__.packages.capfox
  ];
  process.manager.implementation = "process-compose";
  processes = {
    valkey = {
      exec = ''
        mkdir -p ${valkeyRuntimeDir} ${valkeyDataDir}
        rm -f ${valkeyPidFile}
        exec valkey-server .config/xiuxian-artisan-workshop/valkey.conf --tcp-backlog 128 --pidfile ${valkeyPidFile}
      '';
      process-compose = {
        readiness_probe = {
          exec.command = ''
            PIDFILE=${valkeyPidFile}
            if [ ! -s "$PIDFILE" ]; then
              exit 1
            fi

            EXPECTED_PID="$(cat "$PIDFILE")"
            ACTUAL_PID="$(
              valkey-cli -u ${valkeyUrl} info server | awk -F: '/^[[:space:]]*process_id:/ { gsub(/[[:space:]\r]/, "", $2); print $2; exit }'
            )"

            if [ -z "$ACTUAL_PID" ] || [ "$ACTUAL_PID" != "$EXPECTED_PID" ]; then
              exit 1
            fi

            valkey-cli -u ${valkeyUrl} ping
          '';
          initial_delay_seconds = 5;
          period_seconds = 2;
          timeout_seconds = 2;
          failure_threshold = 30;
        };
      };
    };

    carfox.exec = "capfox start";
    agent.exec = "just agent-channel-webhook-restart";

    # Wendao Phase 7.6 Integrated Services
    wendao-gateway = {
      exec = ''
        mkdir -p ${gatewayRuntimeDir}
        rm -f ${gatewayPidFile}
        printf '%s\n' "$$" > ${gatewayPidFile}
        export WENDAO_GATEWAY_PIDFILE=${gatewayPidFile}
        export CARGO_TARGET_DIR=${gatewayTargetDir}
        export VALKEY_URL=redis://127.0.0.1:6379/0
        cargo build -p xiuxian-wendao --bin wendao --locked
        exec ${gatewayTargetDir}/debug/wendao --conf ${gatewayConfig} gateway start
      '';
      process-compose = {
        depends_on = {
          valkey.condition = "process_healthy";
        };
        readiness_probe = {
          exec.command = ''
            PIDFILE=${gatewayPidFile}
            if [ ! -s "$PIDFILE" ]; then
              exit 1
            fi

            EXPECTED_PID="$(cat "$PIDFILE")"
            PORT=$(awk -F= '/^[[:space:]]*port[[:space:]]*=/{gsub(/[[:space:]"]/, "", $2); print $2; exit}' ${gatewayConfig})
            if [ -z "$PORT" ]; then
              PORT=9517
            fi

            RESPONSE="$(
              curl -sS --max-time 2 -D - -o /dev/null "http://127.0.0.1:$PORT/api/health"
            )" || exit 1

            HTTP_STATUS="$(printf '%s\n' "$RESPONSE" | awk 'NR == 1 { print $2; exit }')"
            ACTUAL_PID="$(printf '%s\n' "$RESPONSE" | awk -F': ' 'tolower($1) == "x-wendao-process-id" { gsub(/[[:space:]\r]/, "", $2); print $2; exit }')"

            if [ "$HTTP_STATUS" != "200" ] || [ -z "$ACTUAL_PID" ] || [ "$ACTUAL_PID" != "$EXPECTED_PID" ]; then
              exit 1
            fi
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
