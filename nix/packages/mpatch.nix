{
  lib,
  fetchFromGitHub,
  rustPlatform,
  pkg-config,
  openssl,
  stdenv,
}:

rustPlatform.buildRustPackage rec {
  pname = "mpatch";
  version = "2d540d86213b2048f6483ac24442db88e1424c23";
  src = fetchFromGitHub {
    owner = "Romelium";
    repo = "mpatch";
    rev = version;
    hash = "sha256-2wKDxrAe84BdNC6uIE9i17gv8i3biREbr/XwObuWKTE=";
  };

  cargoHash = "sha256-KnwXbTG9DxWZi0W/N8k4nAeW/STM8ud7UY3h0c+FwDs=";

  nativeBuildInputs = [
    pkg-config
    openssl
  ];

  buildInputs = [
    openssl
  ]
  ++ lib.optionals stdenv.isLinux [ ]
  ++ lib.optionals stdenv.isDarwin [ ];

  doCheck = false;

  meta = with lib; {
    description = "Terminal-based markdown note manager";
    homepage = "https://github.com/Linus-Mussmaecher/rucola";
    license = licenses.gpl3Only;
    maintainers = [ ];
    mainProgram = "iwe";
  };
}
