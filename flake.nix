{
  description = "Profian Drawbridge";

  inputs.nixify.url = github:rvolosatovs/nixify;

  outputs = {nixify, ...}:
    with nixify.lib;
      rust.mkFlake {
        src = ./.;

        ignorePaths = [
          "/.github"
          "/.gitignore"
          "/Drawbridge.toml.example"
          "/Enarx.toml"
          "/flake.lock"
          "/flake.nix"
          "/LICENSE"
          "/README.md"
          "/rust-toolchain.toml"
        ];

        withDevShells = {
          devShells,
          pkgs,
          ...
        }:
          extendDerivations {
            buildInputs = [
              pkgs.openssl
            ];
          }
          devShells;
      };
}
