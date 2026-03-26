{ __inputs__, ... }:
let
  gatewayConfig = "packages/rust/crates/xiuxian-wendao/wendao.toml";
  gatewayTargetDir = ".cache/cargo-target/wendao-gateway-process-compose";
  gatewayRuntimeDir = ".run/wendao-gateway";
  gatewayPidFile = "${gatewayRuntimeDir}/wendao.pid";
  gatewayLogDir = ".run/logs";
  gatewayStdoutLog = "${gatewayLogDir}/wendao-gateway.stdout.log";
  gatewayStderrLog = "${gatewayLogDir}/wendao-gateway.stderr.log";
  sentinelTargetDir = ".cache/cargo-target/wendao-sentinel-process-compose";
  valkeyDataDir = ".data/valkey";
  valkeyRuntimeDir = ".run/valkey";
  valkeyPidFile = "${valkeyRuntimeDir}/valkey.pid";
  valkeyPort = 6379;
  valkeyHost = "127.0.0.1";
in
{
  packages = [
    __inputs__.packages.capfox
  ];
  process.manager.implementation = "process-compose";
  processes = {
    valkey = {
      exec = ''
        ROOT_DIR="$PRJ_ROOT"
        VALKEY_RUNTIME_DIR="$ROOT_DIR/${valkeyRuntimeDir}"
        VALKEY_DATA_DIR="$ROOT_DIR/${valkeyDataDir}"
        VALKEY_PIDFILE="$ROOT_DIR/${valkeyPidFile}"
        mkdir -p "$VALKEY_RUNTIME_DIR" "$VALKEY_DATA_DIR"
        rm -f "$VALKEY_PIDFILE"
        export VALKEY_RUNTIME_DIR="$VALKEY_RUNTIME_DIR"
        export VALKEY_DATA_DIR="$VALKEY_DATA_DIR"
        export VALKEY_PIDFILE="$VALKEY_PIDFILE"
        export VALKEY_PORT=${toString valkeyPort}
        export VALKEY_HOST=${valkeyHost}
        export VALKEY_BIND=${valkeyHost}
        export VALKEY_DAEMONIZE=no
        bash scripts/channel/valkey-launch.sh
      '';
      process-compose = {
        readiness_probe = {
          exec.command = ''
            ROOT_DIR="$PRJ_ROOT"
            export VALKEY_RUNTIME_DIR="$ROOT_DIR/${valkeyRuntimeDir}"
            export VALKEY_DATA_DIR="$ROOT_DIR/${valkeyDataDir}"
            export VALKEY_PIDFILE="$ROOT_DIR/${valkeyPidFile}"
            export VALKEY_PORT=${toString valkeyPort}
            export VALKEY_HOST=${valkeyHost}
            export VALKEY_BIND=${valkeyHost}
            bash scripts/channel/valkey-healthcheck.sh >/dev/null
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
        mkdir -p ${gatewayRuntimeDir} ${gatewayLogDir}
        rm -f ${gatewayPidFile}
        printf '%s\n' "$$" > ${gatewayPidFile}
        export WENDAO_GATEWAY_PIDFILE=${gatewayPidFile}
        export CARGO_TARGET_DIR=${gatewayTargetDir}
        export VALKEY_URL=redis://127.0.0.1:6379/0
        cargo build -p xiuxian-wendao --bin wendao --locked
        exec ${gatewayTargetDir}/debug/wendao --conf ${gatewayConfig} gateway start \
          > >(tee -a ${gatewayStdoutLog}) \
          2> >(tee -a ${gatewayStderrLog} >&2)
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
