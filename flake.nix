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
      {
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
            ];
            buildInputs = [
              graphviz # Broken
              openssl
              sqlite
            ];
          };
      }
    );
}
