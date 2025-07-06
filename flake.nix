{
  description = "a voxel engine";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {self, nixpkgs, devshell, utils, rust-overlay, ...}@inputs:
    utils.lib.eachDefaultSystem (system:
      let
        lib = nixpkgs.lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ devshell.overlays.default (import rust-overlay) ];
        };
      in
      {
        devShells.default = (pkgs.devshell.mkShell {
          name = "voxel-engine";
          packages = with pkgs; [
            # basics
            stdenv.cc
            coreutils

            # rust dev
            rust-bin.stable.latest.default
            rust-analyzer
            cargo-expand

            # vulkan dev
            vulkan-validation-layers

            # some stupid stuff
            gnumake
            cmake
            python3
            pkg-config
            fontconfig.lib
          ];
          env = [
            {
              name = "LD_LIBRARY_PATH";
              value = with pkgs; lib.makeLibraryPath [
                # winit on wayland
                wayland libxkbcommon

                # vulkan
                vulkan-loader

                # idk?
                fontconfig.lib

              ];
            }
            {
              name = "PKG_CONFIG_PATH";
              value = "${pkgs.fontconfig.lib}/lib";
            }
          ];
        });
    });
}
