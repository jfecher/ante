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
            in {
              packages.${system} =
                let
                  ante = craneLib.buildPackage {
                    pname = "ante";
                    version = anteVersion;

                    src = pkgs.lib.fileset.toSource {
                      root = ./.;
                      fileset = pkgs.lib.fileset.unions [
                        ./src
                        ./Cargo.toml
                        ./Cargo.lock
                        ./build.rs
                      ];
                    };

                    nativeBuildInputs = with pkgs; [
                      installShellFiles
                    ];

                    buildInputs = anteDeps ++ [
                      ante-stdlib
                    ];

                    postInstall = ''
                      installShellCompletion --cmd ante \
                        --bash <($out/bin/ante --shell-completion bash) \
                        --fish <($out/bin/ante --shell-completion fish) \
                        --zsh <($out/bin/ante --shell-completion zsh)
                    '';
                  };
                  
                  ante-ls = craneLib.buildPackage {
                    pname = "ante-ls";
                    version = "0.1.1";
                    
                    src = ./ante-ls;

                    ANTE_STDLIB_DIR = "${ante-stdlib}/lib";
                  };

                  ante-stdlib = pkgs.stdenv.mkDerivation {
                    pname = "ante-stdlib";
                    version = anteVersion;

                    src = ./stdlib;

                    dontBuild = true;

                    installPhase = ''
                      mkdir -p $out/lib
                      find . -type f -exec install -Dm644 "{}" -t $out/lib \;
                    '';
                  };
                in {
                  inherit ante ante-ls;
                  default = ante;
                };
              
              devShells.${system}.default = craneLib.devShell {
                buildInputs = anteDeps;

                LLVM_SYS_211_PREFIX = "${pkgs.llvmPackages_21.llvm.dev}";
                NIX_CFLAGS_COMPILE = "-U_FORTIFY_SOURCE";
              };
            })
        ));
}
