# This file is the flake of this project. Nix is a package manager. A flake is
# the file that tells Nix what a repository offers.
#
# This flake offers two things:
#
# - the program `thunderbox`, which a person installs with Nix;
# - a shell that holds each tool that CLAUDE.md section 4 and section 6 name.
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
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;

      # This list holds the systems that this flake supports. CLAUDE.md section
      # 4.4 records the systems that a person built the package on.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      workspace = (lib.importTOML ./Cargo.toml).workspace.package;
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

          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };

          thunderbox = rustPlatform.buildRustPackage {
            pname = "thunderbox";
            inherit (workspace) version;

            # This set holds the files that the build reads. A change to a
            # document, or to a picture, therefore starts no new build.
            #
            # `crates/thunderbox/src/sources.rs` includes two files of
            # `dashboards`. Add a path here when the code includes a file from
            # another directory.
            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./crates
                ./dashboards
              ];
            };

            # Nix reads the hash of each dependency from Cargo.lock. This file
            # therefore holds no hash, and a change to Cargo.lock needs no
            # change here.
            cargoLock.lockFile = ./Cargo.lock;

            # The feature `prometheus` needs the crate `aws-lc-sys`, and that
            # crate holds the C source of the library `aws-lc`. nixpkgs tells
            # the crate to link the `aws-lc` of nixpkgs instead, and the crate
            # returns an error when it finds none. The crate finds the library
            # with `pkg-config`.
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.aws-lc ];

            # The check phase runs `cargo test` for the whole workspace. One
            # test file of `thunderbox-source-prometheus` starts a server on
            # this machine. The sandbox of macOS forbids that without this line.
            __darwinAllowLocalNetworking = true;

            meta = {
              description = "A dashboard platform for the terminal.";
              homepage = workspace.repository;
              license = with lib.licenses; [
                mit
                asl20
              ];
              mainProgram = "thunderbox";
              platforms = systems;
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
        in
        {
          packages.default = thunderbox;

          # `nix flake check` builds the package, and the build runs the tests.
          checks.default = thunderbox;

          # The shell holds the tools alone. It holds no copy of the package, so
          # `nix develop` compiles nothing.
          devShells.default = pkgs.mkShell {
            packages = [
              shellToolchain
              pkgs.python3
              agg
            ];
          };

          # `nix fmt` gives this file one format.
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
      overlays.default = final: _prev: {
        thunderbox = self.packages.${final.stdenv.hostPlatform.system}.default;
      };
    };
}
