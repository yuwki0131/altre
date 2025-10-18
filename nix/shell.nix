{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    rustup
    cargo
    rustc
    pkg-config
    slint
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
  '';
}
