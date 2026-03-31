{
  config,
  lib,
  pkgs,
  ...
}:
let
  mkPath = packages: lib.makeBinPath (lib.filter lib.isDerivation packages);

  pythonBaseEnv = [
    config.languages.python.uv.package
    config.languages.python.package
    pkgs.bash
    pkgs.coreutils
  ];

  pythonScriptEnv = pythonBaseEnv ++ [
    pkgs.just
    pkgs.findutils
    pkgs.gawk
    pkgs.gitMinimal
    pkgs.gnugrep
    pkgs.gnused
  ];

  pythonBenchmarkEnv = pythonScriptEnv ++ [
    pkgs.ripgrep
  ];

  rustBaseEnv = pythonScriptEnv ++ [
    pkgs.ripgrep
    config.languages.rust.toolchainPackage
    pkgs.clang
    pkgs.openssl
    pkgs.pkg-config
    pkgs.protobuf
    pkgs.python3
    pkgs.zlib
  ];

  rustQualityEnv = rustBaseEnv ++ [
    pkgs.cargo-audit
    pkgs.cargo-deny
    pkgs.cargo-nextest
  ];

  rustSecurityEnv = rustBaseEnv ++ [
    pkgs.cargo-audit
    pkgs.cargo-deny
  ];

  rustGovernanceEnv = rustBaseEnv ++ [
    pkgs.cargo-semver-checks
    pkgs.cargo-machete
    pkgs.cargo-udeps
  ];

  # Reuse CI-relevant tool packages from global config, but exclude heavy runtime-only tools.
  ciSupportEnv = lib.filter (
    pkg:
    lib.isDerivation pkg
    && !(lib.elem (lib.getName pkg) [
      "ollama"
      "ngrok"
      "secretspec"
      "valkey"
    ])
  ) config.packages;

  hookEnv = pythonBenchmarkEnv ++ ciSupportEnv;
  pythonTaskEnv = pythonBaseEnv;
  pythonScriptTaskEnv = pythonScriptEnv;
  pythonBenchmarkTaskEnv = pythonBenchmarkEnv;
  rustTaskEnv = rustBaseEnv;
  rustQualityTaskEnv = rustQualityEnv;
  rustSecurityTaskEnv = rustSecurityEnv;
  rustGovernanceTaskEnv = rustGovernanceEnv;
  runtimeTaskEnv = rustBaseEnv ++ [ pkgs.valkey ];

  mkTask = envPackages: command: {
    exec = command;
    env = {
      PATH = "${mkPath envPackages}:$PATH";
    };
  };

  mkRustTaskWith = envPackages: command: {
    exec = ''
      export PKG_CONFIG_PATH="${pkgs.zlib.dev}/lib/pkgconfig:${pkgs.zlib.out}/lib/pkgconfig:''${PKG_CONFIG_PATH:-}"
      ${command}
    '';
    env = {
      PATH = "${mkPath envPackages}:$PATH";
      PROTOC = "${pkgs.protobuf}/bin/protoc";
      PYO3_PYTHON = "${config.languages.python.package}/bin/python";
    };
  };

  mkRustTask = command: mkRustTaskWith rustTaskEnv command;
  mkRustQualityTask = command: mkRustTaskWith rustQualityTaskEnv command;
  mkRustSecurityTask = command: mkRustTaskWith rustSecurityTaskEnv command;
  mkRustGovernanceTask = command: mkRustTaskWith rustGovernanceTaskEnv command;

  mkPythonTask = command: mkTask pythonTaskEnv command;
  mkPythonScriptTask = command: mkTask pythonScriptTaskEnv command;
  mkPythonBenchmarkTask = command: mkTask pythonBenchmarkTaskEnv command;
  mkRuntimeTask = command: mkTask runtimeTaskEnv command;
in
{
  tasks = {
    "ci:architecture-gate" = mkPythonScriptTask ''
      just architecture-gate
    '';

    "ci:lint" = mkTask hookEnv ''
      just lint
    '';

    "ci:check-format" = mkTask hookEnv ''
      just check-format
    '';

    "ci:check-commits" = mkTask hookEnv ''
      just check-commits
    '';

    "ci:rust-quality-gate" = mkRustQualityTask ''
      just rust-quality-gate-ci "''${RUST_CHECK_TIMEOUT_SECS:-3600}"
    '';

    "ci:rust-security-gate" = mkRustSecurityTask ''
      just rust-security-gate
    '';

    "ci:rust-contract-dependency-governance" = mkRustGovernanceTask ''
      just rust-contract-dependency-governance
    '';

    "ci:rust-xiuxian-core-rs-lib" = mkRustTask ''
      just rust-xiuxian-core-rs-lib
    '';

    "ci:rust-xiuxian-daochang-profiles" = mkRustTask ''
      just rust-xiuxian-daochang-profiles
    '';

    "ci:rust-xiuxian-daochang-dependency-assertions" = mkRustTask ''
      just rust-xiuxian-daochang-dependency-assertions
    '';

    "ci:rust-xiuxian-daochang-backend-role-contracts" = mkRustTask ''
      just rust-xiuxian-daochang-backend-role-contracts
    '';

    "ci:rust-xiuxian-daochang-embedding-role-perf-medium-gate" = mkRustTask ''
      just rust-xiuxian-daochang-embedding-role-perf-medium-gate
    '';

    "ci:rust-xiuxian-daochang-embedding-role-perf-heavy-gate" = mkRustTask ''
      just rust-xiuxian-daochang-embedding-role-perf-heavy-gate
    '';

    "ci:rust-fusion-snapshots" = mkRustTask ''
      just rust-fusion-snapshots
    '';

    "ci:rust-search-perf-guard" = mkRustTask ''
      just rust-search-perf-guard
    '';

    "ci:rust-retrieval-audits" = mkRustTask ''
      just rust-retrieval-audits
    '';

    "ci:rust-wendao-performance-gate" = mkRustTask ''
      just rust-wendao-performance-gate
    '';

    "ci:rust-wendao-performance-stress" = mkRustTask ''
      just rust-wendao-performance-stress
    '';

    "ci:rust-wendao-performance-bench" = mkRustTask ''
      just rust-wendao-performance-bench
    '';

    "ci:rust-wendao-performance-bench-fast" = mkRustTask ''
      just rust-wendao-performance-bench-fast
    '';

    "ci:contract-e2e-route-test-json" = mkPythonScriptTask ''
      just contract-e2e-route-test-json
    '';

    "ci:contract-freeze" = mkPythonScriptTask ''
      just test-contract-freeze
    '';

    "ci:docs-vector-search-options-check" = mkPythonScriptTask ''
      just docs-vector-search-options-check
    '';

    "ci:scripts-smoke" = mkPythonScriptTask ''
      just ci-scripts-smoke
    '';

    "ci:test-quick" = mkPythonScriptTask ''
      just test-quick
    '';

    "ci:no-inline-python-guard" = mkPythonScriptTask ''
      just no-inline-python-guard
    '';

    "ci:benchmark-skills-tools" = mkPythonBenchmarkTask ''
      just benchmark-skills-tools-ci \
        "''${OMNI_SKILLS_TOOLS_REPORT_DIR:-.run/reports/skills-tools-benchmark}" \
        "''${OMNI_SKILLS_TOOLS_DETERMINISTIC_RUNS:-3}" \
        "''${OMNI_SKILLS_TOOLS_NETWORK_RUNS:-5}"
    '';

    "ci:tool-list-sweep" = mkPythonScriptTask ''
      just benchmark-tool-list-sweep \
        "''${OMNI_TOOL_LIST_BASE_URL:-}" \
        "''${OMNI_TOOL_LIST_HOST:-}" \
        "''${OMNI_TOOL_LIST_PORT:-}" \
        "''${OMNI_TOOL_LIST_NO_EMBEDDING:-true}" \
        "''${OMNI_TOOL_LIST_HEALTH_TIMEOUT_SECS:-120}" \
        "''${OMNI_TOOL_LIST_TOTAL:-1000}" \
        "''${OMNI_TOOL_LIST_CONCURRENCY_VALUES:-40,80,120,160,200}" \
        "''${OMNI_TOOL_LIST_WARMUP_CALLS:-2}" \
        "''${OMNI_TOOL_LIST_TIMEOUT_SECS:-30}" \
        "''${OMNI_TOOL_LIST_P95_SLO_MS:-400}" \
        "''${OMNI_TOOL_LIST_P99_SLO_MS:-800}" \
        "''${OMNI_TOOL_LIST_STRICT_SNAPSHOT:-true}" \
        "''${OMNI_TOOL_LIST_WRITE_SNAPSHOT:-false}" \
        "''${OMNI_TOOL_LIST_REPORT_DIR:-.run/reports/tool-list-sweep}"
    '';

    "ci:knowledge-recall-gates" = mkPythonScriptTask ''
      just knowledge-recall-perf-ci \
        "''${OMNI_KNOWLEDGE_RECALL_RUNS:-3}" \
        "''${OMNI_KNOWLEDGE_RECALL_WARM_RUNS:-1}" \
        "''${OMNI_KNOWLEDGE_RECALL_QUERY:-x}" \
        "''${OMNI_KNOWLEDGE_RECALL_LIMIT:-2}" \
        "''${OMNI_KNOWLEDGE_RECALL_REPORT_DIR:-.run/reports/knowledge-recall-perf}"
    '';

    "ci:wendao-ppr-gate" = mkPythonScriptTask ''
      just gate-wendao-ppr
    '';

    "ci:wendao-ppr-report" = mkPythonScriptTask ''
      just gate-wendao-ppr-report
    '';

    "ci:wendao-ppr-mixed-canary" = mkPythonScriptTask ''
      just gate-wendao-ppr-mixed-canary
    '';

    "ci:wendao-ppr-report-validate" = mkPythonScriptTask ''
      just validate-wendao-ppr-reports
    '';

    "ci:wendao-ppr-gate-summary" = mkPythonScriptTask ''
      just wendao-ppr-gate-summary
    '';

    "ci:wendao-ppr-rollout-status" = mkPythonScriptTask ''
      just wendao-ppr-rollout-status
    '';

    "ci:memory-gate-quick" = mkRuntimeTask ''
      just memory-gate-quick
    '';

    "ci:memory-gate-nightly" = mkRuntimeTask ''
      just memory-gate-nightly
    '';

    "ci:memory-gate-a7" = mkRuntimeTask ''
      just memory-gate-a7
    '';

    "ci:native-runtime-smoke" = mkPythonScriptTask ''
      just verify-native-runtime
    '';

    "ci:valkey-live" = mkRuntimeTask ''
      just valkey-live
    '';

    "ci:telegram-session-isolation-rust" = mkRustTask ''
      just telegram-session-isolation-rust
    '';

    "ci:telegram-session-isolation-python" = mkPythonScriptTask ''
      just telegram-session-isolation-python
    '';

    "dev:clean-generated" = mkTask hookEnv ''
      just clean-generated
    '';

    "dev:clean-rust" = mkRustTask ''
      just clean-rust
    '';

    "dev:clean-all" = mkRustTask ''
      just clean-all
    '';
  };
}
