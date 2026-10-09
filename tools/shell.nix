{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  packages = with pkgs; [ rustup curl python3 pkg-config openssl cacert ];
  # Official verifier and solver binaries use the native ELF loader on NixOS.
  NIX_LD = "${pkgs.stdenv.cc.bintools.dynamicLinker}";
  NIX_LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib pkgs.openssl ];
}
