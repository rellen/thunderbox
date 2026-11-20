# This file is the flake of this project. Nix is a package manager. A flake is
# the file that tells Nix what a repository offers.
#
# This flake offers two things:
#
# - the program `thunderbox`, which a person installs with Nix. default.nix
#   holds that package;
# - a shell that holds each tool that CLAUDE.md section 4, section 6, and
#   section 9 name.
#
# rust-toolchain.toml names the release of Rust, and this flake reads that file.
# The package, the shell, rustup, and the workflow therefore use one release.
{
  description = "A dashboard platform for the terminal.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    # nixpkgs holds one release of Rust, and that release changes when nixpkgs
    # changes. rust-overlay holds every release, so it supplies the release that
    # rust-toolchain.toml names.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      inherit (nixpkgs) lib;

      # This list holds the systems that this flake supports. CLAUDE.md section
      # 4.4 records the systems that a person or a workflow built the package
      # on.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      pinned = (lib.importTOML ./rust-toolchain.toml).toolchain;

      # The function `outputsOf` makes each output of one system.
      outputsOf =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          rustBin = rust-overlay.lib.mkRustBin { } pkgs;

          # A toolchain is the set of the programs of one release of Rust. This
          # value is the toolchain that rust-toolchain.toml names. It holds the
          # profile and the components of that file. The package needs no more.
          toolchain = rustBin.fromRustupToolchainFile ./rust-toolchain.toml;

          # This value is the toolchain of the shell. An editor needs two more
          # components.
          # `rust-analyzer` is the program that reads the code for an editor,
          # and it reads the source of the standard library from `rust-src`.
          #
          # `override` replaces the list of the components, so this list starts
          # with the components that rust-toolchain.toml names.
          shellToolchain = toolchain.override {
            extensions = (pinned.components or [ ]) ++ [
              "rust-src"
              "rust-analyzer"
            ];
          };

          # default.nix holds the package. This call gives it the toolchain
          # above, so the package and the shell use one release of Rust.
          thunderbox = pkgs.callPackage ./default.nix {
            rustPlatform = pkgs.makeRustPlatform {
              cargo = toolchain;
              rustc = toolchain;
            };
          };

          # `agg` turns a cast file into a picture, and CLAUDE.md section 6
          # gives it `--font-family "Unifont"`. The Nix store is the directory
          # where Nix keeps each package. `agg` searches the fonts of the
          # machine, and it does not search the Nix store. This package
          # therefore gives `agg` the directory of Unifont.
          agg = pkgs.symlinkJoin {
            name = "agg-with-unifont";
            paths = [ pkgs.asciinema-agg ];
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postBuild = ''
              wrapProgram $out/bin/agg \
                --add-flags "--font-dir ${pkgs.unifont}/share/fonts"
            '';
          };

          # The shell holds the tools alone. It holds no copy of the package, so
          # `nix develop` compiles nothing.
          shell = pkgs.mkShell {
            packages = [
              shellToolchain
              pkgs.python3
              agg
              # mdBook builds the documentation site. CLAUDE.md section 9 gives
              # the commands.
              pkgs.mdbook
            ];
          };
        in
        {
          packages.default = thunderbox;
          devShells.default = shell;

          # `nix flake check` builds each check. The first check is the package,
          # and the build of the package runs the tests. The second check is the
          # shell, so a fault of the shell also stops that command.
          checks = {
            default = thunderbox;
            inherit shell;
          };

          # `nix fmt` gives one format to each Nix file of this repository.
          formatter = pkgs.nixfmt-tree;
        };

      # Nix reads the type of an output first and the system second, as in
      # `packages.x86_64-linux`. This function puts the outputs of each system
      # into that shape.
      forEachSystem = name: lib.genAttrs systems (system: (outputsOf system).${name});
    in
    {
      packages = forEachSystem "packages";
      checks = forEachSystem "checks";
      devShells = forEachSystem "devShells";
      formatter = forEachSystem "formatter";

      # An overlay adds a package to the package set of another configuration.
      # A configuration that takes this overlay holds `pkgs.thunderbox`.
      #
      # That package comes from the nixpkgs of the configuration, and it uses
      # the release of Rust of that nixpkgs. Cargo returns an error when that
      # release is older than the field `rust-version` of Cargo.toml.
      overlays.default = final: _prev: {
        thunderbox = final.callPackage ./default.nix { };
      };
    };
}
