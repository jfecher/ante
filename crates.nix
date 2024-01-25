{
  perSystem = { pkgs, lib, config, ... }: {
    nci =
      let
        llvmPackages = pkgs.llvmPackages_16;
        major = lib.versions.major llvmPackages.llvm.version;
        minor = lib.versions.minor llvmPackages.llvm.version;
        llvm-sys-ver = "${major}${builtins.substring 0 1 minor}";
        env = { "LLVM_SYS_${llvm-sys-ver}_PREFIX" = llvmPackages.llvm.dev; };

        stdlib = pkgs.stdenv.mkDerivation {
          pname = "ante-stdlib";
          version = config.packages.ante.version;
          src = ./stdlib;
          phases = [ "unpackPhase" "installPhase" ];
          installPhase = ''
            find . -type f -exec install -Dm644 "{}" -t $out/lib \;
          '';
        };
      in
      {
        toolchainConfig = {
          channel = "stable";
          components = [ "rust-analyzer" "clippy" "rustfmt" "rust-src" ];
        };

        projects.ante = {
          export = false;
          path = ./.;
        };

        crates = {
          ante-ls = {
            profiles.release.features = [ ];
            drvConfig.mkDerivation = {
              preBuild = ''
                export ANTE_STDLIB_DIR=${stdlib}/lib
              '';
            };
          };

          ante = {
            depsDrvConfig = {
              inherit env;
            };
            drvConfig = {
              inherit env;
              mkDerivation = {
                nativeBuildInputs = [ pkgs.installShellFiles ];
                buildInputs = lib.attrValues
                  {
                    inherit (pkgs)
                      libffi
                      libxml2
                      ncurses;
                  } ++ [ llvmPackages.llvm stdlib ];

                postPatch = ''
                  substituteInPlace tests/golden_tests.rs --replace \
                    'target/debug' "target/$(rustc -vV | sed -n 's|host: ||p')/release"
                '';

                preBuild = ''
                  export ANTE_STDLIB_DIR=${stdlib}/lib
                '';

                postInstall = ''
                  installShellCompletion --cmd ante \
                    --bash <($out/bin/ante --shell-completion bash) \
                    --fish <($out/bin/ante --shell-completion fish) \
                    --zsh <($out/bin/ante --shell-completion zsh)
                '';
              };
            };
          };
        };
      };
  };
}
