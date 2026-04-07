{ __inputs__, ... }:
let
  gatewayConfig = "wendao.toml";
  gatewayPortResolver = "scripts/channel/resolve_wendao_gateway_port.py";
  gatewayTargetDir = ".cache/cargo-target/wendao-gateway-process-compose";
  gatewayRuntimeDir = ".run/wendao-gateway";
  gatewayPidFile = "${gatewayRuntimeDir}/wendao.pid";
  gatewayLogDir = ".run/logs";
  gatewayStdoutLog = "${gatewayLogDir}/wendao-gateway.stdout.log";
  gatewayStderrLog = "${gatewayLogDir}/wendao-gateway.stderr.log";
  sentinelTargetDir = ".cache/cargo-target/wendao-sentinel-process-compose";
  sentinelRuntimeDir = ".run/wendao-sentinel";
  sentinelPidFile = "${sentinelRuntimeDir}/wendao-sentinel.pid";
  valkeyDataDir = ".data/valkey";
  valkeyRuntimeDir = ".run/valkey";
  valkeyPidFile = "${valkeyRuntimeDir}/valkey.pid";
  valkeyPort = 6379;
  valkeyHost = "127.0.0.1";
  wendaosearchRuntimeDir = ".run/wendaosearch";
  wendaosearchLogDir = ".run/logs";
  wendaosearchSolverDemoConfig = ".data/WendaoSearch.jl/config/live/solver_demo.toml";
  wendaosearchSolverDemoStdoutLog = "${wendaosearchLogDir}/wendaosearch-solver-demo.stdout.log";
  wendaosearchSolverDemoStderrLog = "${wendaosearchLogDir}/wendaosearch-solver-demo.stderr.log";
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
        ROOT_DIR="$PRJ_ROOT"
        GATEWAY_CONFIG="$ROOT_DIR/${gatewayConfig}"
        mkdir -p ${gatewayRuntimeDir} ${gatewayLogDir}
        rm -f ${gatewayPidFile}
        export CARGO_TARGET_DIR=${gatewayTargetDir}
        export VALKEY_URL=redis://127.0.0.1:6379/0
        cargo build -p xiuxian-wendao --bin wendao --locked
        ${gatewayTargetDir}/debug/wendao --conf "$GATEWAY_CONFIG" gateway start \
          > >(tee -a ${gatewayStdoutLog}) \
          2> >(tee -a ${gatewayStderrLog} >&2) &
        GATEWAY_CHILD_PID=$!
        printf '%s\n' "$GATEWAY_CHILD_PID" > ${gatewayPidFile}
        export WENDAO_GATEWAY_PIDFILE=${gatewayPidFile}
        trap 'kill "$GATEWAY_CHILD_PID" 2>/dev/null || true' TERM INT
        wait "$GATEWAY_CHILD_PID"
      '';
      process-compose = {
        depends_on = {
          valkey.condition = "process_healthy";
        };
        readiness_probe = {
          exec.command = ''
            ROOT_DIR="$PRJ_ROOT"
            PIDFILE="$ROOT_DIR/${gatewayPidFile}"
            if [ ! -s "$PIDFILE" ]; then
              exit 1
            fi
            GATEWAY_CONFIG="$ROOT_DIR/${gatewayConfig}"
            PORT="$(python3 "$ROOT_DIR/${gatewayPortResolver}" --config "$GATEWAY_CONFIG")" || exit 1
            python3 "$ROOT_DIR/scripts/channel/check_wendao_gateway_health.py" \
              --host 127.0.0.1 \
              --port "$PORT" \
              --pidfile "$PIDFILE" \
              --timeout-secs 2 >/dev/null
          '';
          initial_delay_seconds = 60;
          period_seconds = 5;
          timeout_seconds = 2;
          failure_threshold = 48;
        };
      };
    };

    wendao-sentinel = {
      exec = ''
        ROOT_DIR="$PRJ_ROOT"
        GATEWAY_CONFIG="$ROOT_DIR/${gatewayConfig}"
        SENTINEL_RUNTIME_DIR="$ROOT_DIR/${sentinelRuntimeDir}"
        SENTINEL_PIDFILE="$ROOT_DIR/${sentinelPidFile}"
        mkdir -p "$SENTINEL_RUNTIME_DIR"
        rm -f "$SENTINEL_PIDFILE"
        export CARGO_TARGET_DIR=${sentinelTargetDir}
        export VALKEY_URL=redis://127.0.0.1:6379/0
        cargo build -p xiuxian-wendao --bin wendao --locked
        ${sentinelTargetDir}/debug/wendao --conf "$GATEWAY_CONFIG" sentinel watch &
        SENTINEL_CHILD_PID=$!
        printf '%s\n' "$SENTINEL_CHILD_PID" > "$SENTINEL_PIDFILE"
        trap 'kill "$SENTINEL_CHILD_PID" 2>/dev/null || true' TERM INT
        wait "$SENTINEL_CHILD_PID"
      '';
      process-compose = {
        depends_on = {
          wendao-gateway.condition = "process_healthy";
        };
        readiness_probe = {
          exec.command = ''
            ROOT_DIR="$PRJ_ROOT"
            GATEWAY_CONFIG="$ROOT_DIR/${gatewayConfig}"
            SENTINEL_PIDFILE="$ROOT_DIR/${sentinelPidFile}"
            python3 "$ROOT_DIR/scripts/channel/check_wendao_sentinel_health.py" \
              --project-root "$ROOT_DIR" \
              --config "$GATEWAY_CONFIG" \
              --pidfile "$SENTINEL_PIDFILE" >/dev/null
          '';
          initial_delay_seconds = 10;
          period_seconds = 5;
          timeout_seconds = 2;
          failure_threshold = 12;
        };
      };
    };

    wendaosearch-solver-demo = {
      exec = ''
        ROOT_DIR="$PRJ_ROOT"
        mkdir -p "$ROOT_DIR/${wendaosearchRuntimeDir}" "$ROOT_DIR/${wendaosearchLogDir}"
        export WENDAOSEARCH_SERVICE_NAME=wendaosearch-solver-demo
        export WENDAOSEARCH_RUNTIME_DIR=${wendaosearchRuntimeDir}
        export WENDAOSEARCH_CONFIG=${wendaosearchSolverDemoConfig}
        bash "$ROOT_DIR/scripts/channel/wendaosearch-launch.sh" \
          > >(tee -a "$ROOT_DIR/${wendaosearchSolverDemoStdoutLog}") \
          2> >(tee -a "$ROOT_DIR/${wendaosearchSolverDemoStderrLog}" >&2)
      '';
      process-compose = {
        readiness_probe = {
          exec.command = ''
            ROOT_DIR="$PRJ_ROOT"

            export WENDAOSEARCH_SERVICE_NAME=wendaosearch-solver-demo
            export WENDAOSEARCH_RUNTIME_DIR=${wendaosearchRuntimeDir}
            export WENDAOSEARCH_CONFIG=${wendaosearchSolverDemoConfig}
            bash "$ROOT_DIR/scripts/channel/wendaosearch-healthcheck.sh" >/dev/null
          '';
          initial_delay_seconds = 5;
          period_seconds = 2;
          timeout_seconds = 3;
          failure_threshold = 90;
        };
      };
    };
  };
}
