# This file is the package of the program `thunderbox`. It holds one function,
# and nixpkgs gives that function each argument through `callPackage`.
#
# flake.nix calls this function two times:
#
# - The output `packages.default` gives it a `rustPlatform` that holds the
#   release of Rust that rust-toolchain.toml names.
# - The output `overlays.default` gives it the nixpkgs of the configuration that
#   takes the overlay. That configuration then compiles the program with its own
#   release of Rust.
#
# A person can replace an argument with `override`. This expression builds the
# program with another `rustPlatform`:
#
#     packages.SYSTEM.default.override { rustPlatform = another; }
{
  lib,
  rustPlatform,
  pkg-config,
  aws-lc,
  cacert,
}:
let
  workspace = (lib.importTOML ./Cargo.toml).workspace.package;
in
rustPlatform.buildRustPackage {
  pname = "thunderbox";
  inherit (workspace) version;

  # This set holds the files that the build reads. A change to a document, or to
  # a picture, therefore starts no new build.
  #
  # `crates/thunderbox/src/sources.rs` includes two files of `dashboards`. Add a
  # path here when the code includes a file from another directory.
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./crates
      ./dashboards
    ];
  };

  # Nix reads the hash of each dependency from Cargo.lock. This file therefore
  # holds no hash, and a change to Cargo.lock needs no change here.
  cargoLock.lockFile = ./Cargo.lock;

  # The feature `prometheus` needs the crate `aws-lc-sys`, and that crate holds
  # the C source of the library `aws-lc`. nixpkgs tells the crate to link the
  # `aws-lc` of nixpkgs instead, and the crate returns an error when it finds
  # none. The crate finds the library with `pkg-config`.
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ aws-lc ];

  # A CA is a certificate authority. A client of the data source prometheus
  # reads the certificates of each CA from the machine, and it returns an error
  # when it finds none. It does so even for an address of plain HTTP. The
  # sandbox holds no such certificate, so 22 tests failed there. The setup hook
  # of `cacert` gives the check phase a file of certificates through the
  # variable SSL_CERT_FILE. CLAUDE.md section 4.4 records the measurement.
  nativeCheckInputs = [ cacert ];

  # The check phase runs `cargo test` for the whole workspace. One test file of
  # `thunderbox-source-prometheus` starts a server on this machine. The sandbox
  # of macOS forbids that without this line.
  __darwinAllowLocalNetworking = true;

  meta = {
    description = "A dashboard platform for the terminal.";
    homepage = workspace.repository;
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "thunderbox";
  };
}
