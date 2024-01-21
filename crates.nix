{ ... }: {
  perSystem = { pkgs, lib, ... }: {
    nci =
      let
        llvmPackages = pkgs.llvmPackages_16;
        major = lib.versions.major llvmPackages.llvm.version;
        minor = lib.versions.minor llvmPackages.llvm.version;
        llvm-sys-ver = "${major}${builtins.substring 0 1 minor}";
        env = {
          "LLVM_SYS_${llvm-sys-ver}_PREFIX" = llvmPackages.llvm.dev;
        };
      in
      {
        toolchainConfig = {
          channel = "stable";
          components = [ "rust-analyzer" "clippy" "rustfmt" "rust-src" ];
        };
        projects.ante.path = ./.;
        crates.ante = {
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
                } ++ [ llvmPackages.llvm ];

              postPatch = ''
                substituteInPlace tests/golden_tests.rs --replace \
                  'target/debug' "target/$(rustc -vV | sed -n 's|host: ||p')/release"
              '';

              preBuild = ''
                export ANTE_STDLIB_DIR=$out/lib
                find stdlib -type f -exec install -Dm644 "{}" -t $ANTE_STDLIB_DIR \;
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
}
