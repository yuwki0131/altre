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
    glib
    pango
    gtk3
    gdk-pixbuf
    cairo
    atk
    libsoup_3
    webkitgtk_4_1
    libappindicator-gtk3
    openssl
    zlib
  ];

  shellHook = ''
    export ALTRE_DEV_SHELL=1
    export PKG_CONFIG_PATH=${pkgs.lib.makeSearchPath "lib/pkgconfig" [
      pkgs.gtk3
      pkgs.gdk-pixbuf
      pkgs.pango
      pkgs.cairo
      pkgs.atk
      pkgs.glib
      pkgs.libsoup_3
      pkgs.webkitgtk_4_1
      pkgs.libappindicator-gtk3
      pkgs.openssl
    ]}:$PKG_CONFIG_PATH
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
      pkgs.glib
      pkgs.pango
      pkgs.gtk3
      pkgs.gdk-pixbuf
      pkgs.cairo
      pkgs.atk
      pkgs.libsoup_3
      pkgs.webkitgtk_4_1
      pkgs.libappindicator-gtk3
      pkgs.openssl
      pkgs.zlib
    ]}:$LD_LIBRARY_PATH
  '';
}
