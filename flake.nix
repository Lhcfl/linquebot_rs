{
  description = "Linquebot's nix flake";

  nixConfig = {
    experimental-features = [
      "nix-command"
      "flakes"
    ];

    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      rec {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              graphviz
              (rust-bin.selectLatestNightlyWith (
                toolchain:
                toolchain.default.override {
                  extensions = [ "rust-src" ];
                }
              ))
            ];
          };
        packages.default =
          with pkgs;
          let
            rustPlatform = makeRustPlatform {
              cargo = rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal);
              rustc = rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal);
            };
          in
          rustPlatform.buildRustPackage {
            pname = "linquebot_rs";
            version = "0.1.0";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = [
              pkg-config
              makeWrapper
            ];
            buildInputs = [
              openssl
            ];
            installPhase = ''
              runHook preInstall
              mkdir -p $out/bin $out/lib
              cargo build --release
              cp target/release/linquebot_rs $out/lib/linquebot_rs
              makeWrapper $out/lib/linquebot_rs $out/bin/linquebot_rs --prefix PATH : ${lib.makeBinPath [ graphviz ]}
              runHook postInstall
            '';
            meta.mainProgram = "linquebot_rs";
          };
        packages.dockerImage =
          with pkgs;
          dockerTools.buildLayeredImage {
            name = "linquebot_rs";
            tag = "latest";
            contents = [
              cacert
            ];
            config.Cmd = [
              "${lib.meta.getExe packages.default}"
            ];
            created = "now";
          };
      }
    );
}
