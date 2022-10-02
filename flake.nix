{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    {
      overlays.default = _: prev:
        { ante = (import ./.) { pkgs = prev; }; };
    } //
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ self.overlays.default ];
        };
        inherit (pkgs) ante mkShell;
      in
      {
        packages = {
          inherit ante;
          default = ante;
        };
        devShells.default = mkShell {
          name = "ante-dev";
          inputsFrom = [ ante ];
          shellHook = ante.shellHook + ''
            export PATH=$PWD/target/debug:$PATH
          '';
        };
      }));
}
