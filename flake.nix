{
  inputs = {
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ parts, nci, ... }:
    parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      imports = [
        parts.flakeModules.easyOverlay
        nci.flakeModule
        ./crates.nix
      ];

      perSystem = { config, ... }:
        let crateOutputs = config.nci.outputs.ante; in
        {
          overlayAttrs.ante = config.packages.default;
          packages.default = crateOutputs.packages.release;
          devShells.default = crateOutputs.devShell.overrideAttrs (_: {
            shellHook = ''
              PATH=$PATH:$PWD/target/debug
            '';
          });
        };
    };
}
