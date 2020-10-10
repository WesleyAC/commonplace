{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    buildInputs = [ pkgs.pkgconfig pkgs.webkitgtk pkgs.glib-networking ];
}

