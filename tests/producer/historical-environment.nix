# Select this exact revision through NIX_PATH or pass -I nixpkgs=/path/to/source.
let
  pkgsSource = <nixpkgs>;
  revision = builtins.replaceStrings [ "\n" ] [ "" ] (builtins.readFile (pkgsSource + "/.git-revision"));
  expectedRevision = "774debe7a0d1b496e35677ad955a1011c6ff74f3";
  pkgs = import pkgsSource {};
  python = pkgs.python312.withPackages (ps: [ ps.fasteners ps.future ]);
in
assert revision == expectedRevision;
pkgs.mkShell {
  packages = [ python pkgs.gcc pkgs.librsync pkgs.pkg-config ];
}
