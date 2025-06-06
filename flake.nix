{
  description = "Linquebot's nix flake";

  nixConfig = {
    experimental-features = [
      "nix-command"
      "flakes"
    ];
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://beiyanyunyi.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "beiyanyunyi.cachix.org-1:iCC1rwPPRGilc/0OS7Im2mP6karfpptTCnqn9sPtwls="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (import rust-overlay)
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal)
        );

      in
      rec {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              openssl
              graphviz
              (rust-bin.selectLatestNightlyWith (
                toolchain:
                toolchain.default.override {
                  extensions = [
                    "rust-src"
                    "clippy-preview"
                  ];
                }
              ))
            ];
          };
        packages.default =
          with pkgs;
          let
            jsonFilter = path: _type: builtins.match ".*json$" path != null;
            jsonOrCargo = path: type: (jsonFilter path type) || (craneLib.filterCargoSources path type);
          in
          craneLib.buildPackage {
            src = lib.cleanSourceWith {
              src = ./.;
              filter = jsonOrCargo;
              name = "source";
            };
            # Add extra inputs here or any other derivation settings
            # doCheck = true;
            buildInputs =
              [ onnxruntime ]
              ++ lib.optionals stdenvNoCC.hostPlatform.isLinux [
                openssl
                pkg-config
              ];
            nativeBuildInputs = [ makeWrapper ];
            postInstall = ''
              wrapProgram $out/bin/linquebot_rs --prefix PATH : ${lib.makeBinPath [ graphviz ]}
            '';
            meta.mainProgram = "linquebot_rs";
          };
        packages.dockerSupports =
          with pkgs;
          let
            fonts-conf = makeFontsConf {
              fontDirectories = [
                twemoji-color-font
                noto-fonts-cjk-sans
                noto-fonts
              ];
            };
          in
          stdenvNoCC.mkDerivation {
            name = "linquebot_rs-docker-supports";
            dontUnpack = true;
            buildInputs = [
              fontconfig
            ];
            installPhase = ''
              runHook preInstall
              mkdir -p $out/etc/fonts/conf.d $out/var
              mkdir -m 1777 $out/tmp
              cp ${fonts-conf} $out/etc/fonts/conf.d/99-nix.conf
              runHook postInstall
            '';
          };
        packages.dockerImage =
          with pkgs;
          dockerTools.buildLayeredImage {
            name = "ghcr.io/lhcfl/linquebot_rs";
            tag = "latest";
            contents = [
              # coreutils
              dockerTools.caCertificates
              dockerTools.usrBinEnv
              # dockerTools.binSh
              # strace
              packages.dockerSupports
            ];
            config = {
              Cmd = [
                "${lib.meta.getExe packages.default}"
              ];
              WorkingDir = "/app";
            };
            created = "now";
          };
      }
    );
}
