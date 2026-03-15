{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "capfox";
  version = "0.3.2";

  src = fetchFromGitHub {
    owner = "haskel";
    repo = "capfox";
    rev = "v${version}";
    sha256 = "sha256-FUSJ4Xs24Cst2ytky9c+kT3fu3iFoFoGGYuFe8UwvjE=";
  };

  vendorHash = "sha256-hctUwVOkkio1M6tPnfpzqr+ANQDNijT6s7Grqa8u0L4=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "HAProxy Exporter for the Prometheus monitoring system";
    mainProgram = "haproxy_exporter";
    homepage = "https://github.com/prometheus/haproxy_exporter";
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ benley ];
  };
}
