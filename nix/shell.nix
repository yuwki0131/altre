{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    rustup
    cargo
    rustc
    pkg-config
    wayland
    wayland-protocols
    libxkbcommon
    fontconfig
    freetype
    harfbuzz
    mesa
    libGL
    vulkan-loader
  ];

  shellHook = ''
    export ALTRE_DEV_SHELL=1
    export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [
      pkgs.wayland
      pkgs.wayland-protocols
      pkgs.libxkbcommon
      pkgs.fontconfig
      pkgs.freetype
      pkgs.harfbuzz
      pkgs.mesa
      pkgs.libGL
      pkgs.vulkan-loader
    ]}:$LD_LIBRARY_PATH
  '';
}
