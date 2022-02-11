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
        apiSpec = "api/api.yml";
        docOutput = "doc/index.html";

        pkgs = import nixpkgs { };

        rust = fenix.packages."${system}".fromToolchainFile { file = ./rust-toolchain.toml; };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            rust

            pkgs.redoc-cli

            (pkgs.writeShellScriptBin "build-doc" ''
              ${pkgs.redoc-cli}/bin/redoc-cli bundle "${apiSpec}" -o "${docOutput}"
            '')
            (pkgs.writeShellScriptBin "watch-doc" ''
              ${pkgs.fd}/bin/fd | ${pkgs.ripgrep}/bin/rg 'api.yml' | ${pkgs.entr}/bin/entr -rs '${pkgs.redoc-cli}/bin/redoc-cli serve "${apiSpec}"'
            '')
          ];
        };
      }
    );
}
