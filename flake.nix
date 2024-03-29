{
  description = "cs128h-project";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-crate2nix = {
      url = "github:kolloch/crate2nix";
      flake = false;
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-parts,
    rust-overlay,
    rust-crate2nix,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
      ];
      perSystem = {
        system,
        pkgs,
        ...
      }: let
        name = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.name;
        overlays = [
          (import rust-overlay)
          (self: super: let
            toolchain = super.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          in {
            rustc = toolchain;
          })
        ];
        pkgs = import nixpkgs {inherit system overlays;};
        # Main build target
        project = let
          crateTools = pkgs.callPackage "${rust-crate2nix}/tools.nix" {inherit pkgs;};
        in
          import (crateTools.generatedCargoNix {
            inherit name;
            src = ./.;
          }) {
            inherit pkgs;
          };
      in rec {
        packages.default = project.rootCrate.build;
        devShells.default =
          project.rootCrate.build
          // pkgs.mkShell {
            inherit name;
            packages = with pkgs; [
              cargo
              cargo-audit
              clippy
              rust-analyzer
              rustc # this is to get rust-src for lsp hints in std
              rustfmt
            ];
          };
      };
    };
}
