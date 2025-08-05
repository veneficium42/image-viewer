{
  description = "i dont know what I'm doing";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      with pkgs;
      {
        devShells.default = mkShell rec {

          packages = [
            cargo-bloat
            upx
            pkg-config
          ];

          buildInputs = [
            rust-bin.stable.latest.default
            libxkbcommon
            wayland
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      }
    );
}
