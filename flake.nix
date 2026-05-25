{
  description = "A safe, easy systems language";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, crane, rust-overlay }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = function: nixpkgs.lib.genAttrs systems (system:
        function (import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        }));
    in
      nixpkgs.lib.foldAttrs nixpkgs.lib.mergeAttrs {} (
        nixpkgs.lib.mapAttrsToList (_: v: v) (
          forAllSystems (pkgs:
            let
              system = pkgs.stdenv.hostPlatform.system;

              craneLib = (crane.mkLib pkgs).overrideToolchain (
                p:
                p.rust-bin.stable.latest.default
              );

              anteDeps = with pkgs; [
                llvmPackages_21.llvm
                libffi
                libxml2
                ncurses
              ];

              anteVersion = "0.1.0";

              ante-stdlib = pkgs.stdenv.mkDerivation {
                pname = "ante-stdlib";
                version = anteVersion;

                src = ./stdlib;

                dontBuild = true;

                installPhase = ''
                  mkdir -p $out/stdlib
                  cp -r src $out/stdlib/src
                '';
              };

              ante-minicoro = pkgs.stdenv.mkDerivation {
                pname = "ante-minicoro";
                version = anteVersion;

                src = ./aminicoro;

                dontBuild = true;

                installPhase = ''
                  install -Dm644 minicoro.c $out/minicoro.c
                  install -Dm644 minicoro.h $out/minicoro.h
                '';
              };

              workspaceSrc = pkgs.lib.fileset.toSource {
                root = ./.;
                fileset = pkgs.lib.fileset.unions [
                  ./src
                  ./ante-ls
                  ./Cargo.toml
                  ./Cargo.lock
                  ./build.rs
                ];
              };

              testSrc = pkgs.lib.fileset.toSource {
                root = ./.;
                fileset = pkgs.lib.fileset.unions [
                  ./src
                  ./ante-ls
                  ./tests
                  ./examples
                  ./Cargo.toml
                  ./Cargo.lock
                  ./build.rs
                ];
              };

              commonEnv = {
                ANTE_STDLIB_DIR = "${ante-stdlib}/stdlib";
                ANTE_MINICORO_PATH = "${ante-minicoro}/minicoro.c";
                LLVM_SYS_211_PREFIX = "${pkgs.llvmPackages_21.llvm.dev}";
              };

              cargoArtifacts = craneLib.buildDepsOnly ({
                src = workspaceSrc;
                buildInputs = anteDeps;
              } // commonEnv);
            in {
              packages.${system} =
                let
                  ante = craneLib.buildPackage ({
                    pname = "ante";
                    version = anteVersion;

                    src = workspaceSrc;
                    inherit cargoArtifacts;
                    cargoExtraArgs = "-p ante";

                    nativeBuildInputs = with pkgs; [
                      installShellFiles
                      makeWrapper
                    ];

                    buildInputs = anteDeps;

                    postInstall = ''
                      mkdir -p $out/share/ante/aminicoro
                      cp ${ante-minicoro}/minicoro.c $out/share/ante/aminicoro/minicoro.c
                      cp ${ante-minicoro}/minicoro.h $out/share/ante/aminicoro/minicoro.h

                      installShellCompletion --cmd ante \
                        --bash <($out/bin/ante --shell-completion bash) \
                        --fish <($out/bin/ante --shell-completion fish) \
                        --zsh <($out/bin/ante --shell-completion zsh)

                      wrapProgram $out/bin/ante \
                        --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.stdenv.cc ]}
                    '';
                  } // commonEnv // {
                    ANTE_MINICORO_PATH = "${placeholder "out"}/share/ante/aminicoro/minicoro.c";
                  });

                  ante-ls = craneLib.buildPackage ({
                    pname = "ante-ls";
                    version = "0.1.1";

                    src = workspaceSrc;
                    inherit cargoArtifacts;
                    cargoExtraArgs = "-p ante-ls";

                    buildInputs = anteDeps;
                  } // commonEnv);
                in {
                  inherit ante ante-ls ante-stdlib ante-minicoro;
                  default = ante;
                };

              checks.${system} = {
                inherit (self.packages.${system}) ante ante-ls;

                ante-tests = craneLib.cargoTest ({
                  pname = "ante-tests";
                  version = anteVersion;

                  src = testSrc;
                  inherit cargoArtifacts;
                  cargoExtraArgs = "-p ante";

                  buildInputs = anteDeps;
                  nativeBuildInputs = [ pkgs.stdenv.cc ];
                } // commonEnv);
              };

              devShells.${system}.default = craneLib.devShell {
                buildInputs = anteDeps;

                LLVM_SYS_211_PREFIX = "${pkgs.llvmPackages_21.llvm.dev}";
                NIX_CFLAGS_COMPILE = "-U_FORTIFY_SOURCE";
              };
            })
        ));
}
