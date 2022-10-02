let
  ante =
    { lib
    , libffi
    , libxml2
    , llvmPackages
    , ncurses
    , rustPlatform
    }:

    let
      major = lib.versions.major llvmPackages.llvm.version;
      minor = lib.versions.minor llvmPackages.llvm.version;
      llvm-sys-ver = "${major}${builtins.substring 0 1 minor}";
      toml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    in

    rustPlatform.buildRustPackage {
      pname = "ante";
      src = ./.;
      inherit (toml.package) version;
      cargoSha256 = "WVNBk/5Q4tpMMQDNgRSSD4WFTOqgCPxa1YkBezqTaRI=";

      nativeBuildInputs = [ llvmPackages.llvm ];
      buildInputs = [ libffi libxml2 ncurses ];

      postPatch = ''
        substituteInPlace tests/golden_tests.rs --replace \
          'target/debug' "target/$(rustc -vV | sed -n 's|host: ||p')/release"
      '';

      shellHook = ''
        export LLVM_SYS_${llvm-sys-ver}_PREFIX=${llvmPackages.llvm.dev}
      '';

      preBuild = ''
        $shellHook
        export ANTE_STDLIB_DIR=$out/lib
        mkdir -p $ANTE_STDLIB_DIR
        cp -r $src/stdlib/* $ANTE_STDLIB_DIR
      '';
    };
in
{ pkgs ? import <nixpkgs> { } }: with pkgs;
callPackage ante { llvmPackages = llvmPackages_13; }
