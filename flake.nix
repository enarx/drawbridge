{
  description = "Profian Drawbridge";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/master;
  inputs.flake-compat.flake = false;
  inputs.flake-compat.url = github:edolstra/flake-compat;
  inputs.flake-utils.url = github:numtide/flake-utils;
  inputs.fenix.inputs.nixpkgs.follows = "nixpkgs";
  inputs.fenix.url = github:nix-community/fenix;

  outputs = { self, nixpkgs, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        rust = fenix.packages."${system}".fromToolchainFile {
          file = ./rust-toolchain.toml;
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            rust
          ];
        };
      }
    );
}
