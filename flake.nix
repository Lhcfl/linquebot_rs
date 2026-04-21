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
    systems.url = "github:nix-systems/default";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { lib, ... }:
      {
        systems = import inputs.systems;
        perSystem =
          { self', system, ... }:
          let
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ inputs.rust-overlay.overlays.default ];
            };
          in
          {
            _module.args.pkgs = pkgs;
            devShells.default = pkgs.mkShell {
              LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
              BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.libclang.lib}/lib/clang/${lib.versions.major (lib.getVersion pkgs.libclang)}/include ${lib.fileContents "${pkgs.stdenv.cc}/nix-support/libc-cflags"}";
              buildInputs = [
                pkgs.pkg-config
                pkgs.cmake
                pkgs.graphviz
                (pkgs.rust-bin.selectLatestNightlyWith (
                  toolchain:
                  toolchain.default.override {
                    extensions = [
                      "rust-src"
                      "clippy-preview"
                    ];
                  }
                ))
              ]
              ++ lib.optional pkgs.stdenvNoCC.hostPlatform.isLinux pkgs.openssl;
            };
            packages = {
              default = self'.packages.linquebot_rs;
              linquebot_rs_lm = self'.packages.linquebot_rs.override { lm = true; };
              linquebot_rs =
                let
                  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
                    p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal)
                  );
                in
                lib.makeOverridable (
                  {
                    lm ? false,
                  }:
                  craneLib.buildPackage {
                    src =
                      let
                        jsonFilter = path: _type: builtins.match ".*json$" path != null;
                        jsonOrCargo = path: type: (jsonFilter path type) || (craneLib.filterCargoSources path type);
                      in
                      lib.cleanSourceWith {
                        src = ./.;
                        filter = jsonOrCargo;
                        name = "source";
                      };
                    buildInputs = [
                      pkgs.pkg-config
                      pkgs.cmake
                    ]
                    ++ lib.optional pkgs.stdenvNoCC.hostPlatform.isLinux pkgs.openssl;
                    nativeBuildInputs = [ pkgs.makeWrapper ];
                    CI = "true";
                    stdenv = p: p.clangStdenv;
                    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
                    cargoExtraArgs = lib.optionalString lm "--features lm";
                    postInstall = ''
                      wrapProgram $out/bin/linquebot_rs --prefix PATH : ${lib.makeBinPath [ pkgs.graphviz ]}
                    '';
                    meta.mainProgram = "linquebot_rs";
                  }
                ) { };
              dockerSupports = pkgs.stdenvNoCC.mkDerivation {
                name = "linquebot_rs-docker-supports";
                dontUnpack = true;
                buildInputs = [
                  pkgs.fontconfig
                ];
                installPhase =
                  let
                    fonts-conf = pkgs.makeFontsConf {
                      fontDirectories = with pkgs; [
                        twemoji-color-font
                        noto-fonts-cjk-sans
                        noto-fonts
                      ];
                    };
                  in
                  ''
                    runHook preInstall
                    mkdir -p $out/etc/fonts/conf.d $out/var
                    mkdir -m 1777 $out/tmp
                    cp ${fonts-conf} $out/etc/fonts/conf.d/99-nix.conf
                    runHook postInstall
                  '';
              };
              dockerImage = pkgs.dockerTools.buildLayeredImage {
                name = "ghcr.io/lhcfl/linquebot_rs";
                tag = "latest";
                contents = [
                  # coreutils
                  pkgs.dockerTools.caCertificates
                  pkgs.dockerTools.usrBinEnv
                  # dockerTools.binSh
                  # strace
                  self'.packages.dockerSupports
                ];
                config = {
                  Cmd = [
                    "${lib.meta.getExe self'.packages.default}"
                  ];
                  WorkingDir = "/app";
                  StopSignal = "SIGINT";
                };
                created = "now";
              };
            };
          };
      }
    );
}
