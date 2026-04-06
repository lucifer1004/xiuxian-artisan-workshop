{ workspaceRoot, inputs, ... }:
{
  perSystem =
    {
      pkgs,
      config,
      lib,
      ...
    }:
    let
      # The dumped Metal toolchain
      apple-metal-toolchain =
        pkgs.callPackage ../../packages/apple-metal-toolchain.nix
          { };

      # The native Nixpkgs SDK
      apple-sdk = pkgs.apple-sdk_15;

      # Combine them into a single directory that looks like /Applications/Xcode.app/Contents/Developer
      xcode-combined = pkgs.symlinkJoin {
        name = "xcode-combined";
        paths = [
          apple-metal-toolchain
          apple-sdk
        ];
      };
    in
    {
      _module.args.apple-metal-toolchain = apple-metal-toolchain;

      nci.projects."xiuxian-core-rs" = {
        path = workspaceRoot;
        export = true;
        depsDrvConfig = {
          mkDerivation = {
            buildInputs = [
              pkgs.pkg-config
              pkgs.openssl
            ];
          };
          env = {
            PROTOC = "${pkgs.protobuf}/bin/protoc";
          };
        };
      };
      nci.projects."xiuxian-wendao" = {
        path = workspaceRoot;
        depsDrvConfig = {
          env = {
            PROTOC = "${pkgs.protobuf}/bin/protoc";
          };
        };
      };

      # configure crates
      nci.crates = {
        # "xiuxian-llm" = {
        #   depsDrvConfig = {
        #     mkDerivation.nativeBuildInputs = lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
        #       apple-metal-toolchain
        #       pkgs.xcbuild
        #     ];
        #     mkDerivation.buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
        #       apple-sdk
        #     ];
        #     env = lib.optionalAttrs pkgs.stdenv.hostPlatform.isDarwin {
        #       MISTRALRS_METAL_PRECOMPILE = "1";
        #       # Point DEVELOPER_DIR to the combined symlink forest
        #       DEVELOPER_DIR = "${xcode-combined}";
        #       # Point SDKROOT to the macOS SDK within that forest
        #       SDKROOT = "${xcode-combined}/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
        #     };
        #   };
        # };
      };

      packages.xiuxian-core-rs-python-bindings =
        pkgs.callPackage ../../packages/xiuxian-core-rs.nix
          {
            inherit workspaceRoot;
            cargoDeps =
              config.nci.outputs."xiuxian-core-rs".packages.release.config.rust-cargo-vendor.vendoredSources;
            version = config.nci.outputs."xiuxian-core-rs".packages.release.config.version;
          };
    };
}
