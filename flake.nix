{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    { overlays.default = _: super: { ante = (import ./.) { pkgs = super; }; }; } //
    (flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system}; in {
        packages = rec {
          ante = (import ./.) { inherit pkgs; };
          default = ante;
        };
      }));
}
