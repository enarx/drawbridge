{
  description = "Profian Drawbridge";

  inputs.cargo2nix.inputs.flake-compat.follows = "flake-compat";
  inputs.cargo2nix.inputs.flake-utils.follows = "flake-utils";
  inputs.cargo2nix.inputs.nixpkgs.follows = "nixpkgs";
  inputs.cargo2nix.inputs.rust-overlay.follows = "rust-overlay";
  inputs.cargo2nix.url = github:cargo2nix/cargo2nix;
  inputs.flake-compat.flake = false;
  inputs.flake-compat.url = github:edolstra/flake-compat;
  inputs.flake-utils.url = github:numtide/flake-utils;
  inputs.nixpkgs.url = github:profianinc/nixpkgs;
  inputs.rust-overlay.inputs.flake-utils.follows = "flake-utils";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  inputs.rust-overlay.url = github:oxalica/rust-overlay;

  outputs = {
    self,
    cargo2nix,
    flake-utils,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [cargo2nix.overlays.default];
        };
        pkgsX86_64LinuxMusl = import nixpkgs {
          inherit system;
          crossSystem = {
            config = "x86_64-unknown-linux-musl";
          };
          overlays = [cargo2nix.overlays.default];
        };

        cargo2nixBin = cargo2nix.packages.${system}.cargo2nix;
        devRust = pkgs.rust-bin.fromRustupToolchainFile "${self}/rust-toolchain.toml";

        cargo.toml = builtins.fromTOML (builtins.readFile "${self}/Cargo.toml");

        mkBin = args: pkgs:
          ((pkgs.rustBuilder.makePackageSet ({
                packageFun = import "${self}/Cargo.nix";
                rustVersion = "1.62.1";
                workspaceSrc =
                  pkgs.nix-gitignore.gitignoreRecursiveSource [
                    "*.nix"
                    "*.yml"
                    "/.github"
                    "flake.lock"
                    "LICENSE"
                    "rust-toolchain.toml"
                  ]
                  self;
              }
              // args))
            .workspace
            ."${cargo.toml.package.name}" {})
          .bin;

        mkReleaseBin = mkBin {};

        nativeBin = mkReleaseBin pkgs;
        x86_64LinuxMuslBin = mkReleaseBin pkgsX86_64LinuxMusl;

        mkDebugBin = mkBin {release = false;};

        nativeDebugBin = mkDebugBin pkgs;
        x86_64LinuxMuslDebugBin = mkDebugBin pkgsX86_64LinuxMusl;

        buildImage = bin:
          pkgs.dockerTools.buildImage {
            inherit (cargo.toml.package) name;
            tag = cargo.toml.package.version;
            contents = [
              bin
            ];
            config.Cmd = [cargo.toml.package.name];
            config.Env = ["PATH=${bin}/bin"];
          };
      in {
        formatter = pkgs.alejandra;

        packages = {
          "${cargo.toml.package.name}" = nativeBin;
          "${cargo.toml.package.name}-x86_64-unknown-linux-musl" = x86_64LinuxMuslBin;
          "${cargo.toml.package.name}-x86_64-unknown-linux-musl-oci" = buildImage x86_64LinuxMuslBin;

          "${cargo.toml.package.name}-debug" = nativeDebugBin;
          "${cargo.toml.package.name}-debug-x86_64-unknown-linux-musl" = x86_64LinuxMuslDebugBin;
          "${cargo.toml.package.name}-debug-x86_64-unknown-linux-musl-oci" = buildImage x86_64LinuxMuslDebugBin;
        };
        packages.default = nativeBin;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.openssl

            cargo2nixBin

            devRust
          ];
        };
      }
    );
}
