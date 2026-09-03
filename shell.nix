{
  pkgs ? import <nixpkgs> { },
}:
let
  inherit (pkgs) lib;

  fonts = with pkgs; [
    (callPackage ./fonts/serif.nix { })
    (callPackage ./fonts/sans-serif.nix { })
    (callPackage ./fonts/typewriter.nix { })
    (callPackage ./fonts/menlo.nix { })
  ];

  texenv = pkgs.texliveMinimal.withPackages (
    p: with p; [
      scheme-basic
      xetex
      biber

      latexmk
      latexindent
      lacheck

      cyrillic
      babel-russian
      hyphen-russian
      ruhyphen

      amsmath
      biblatex
      bussproofs
      csquotes
      fancyhdr
      fontspec
      framed
      geometry
      hyperref
      lineno
      mdframed
      minted
      needspace
      polyglossia
      sourcecodepro
      unicode-math
      upquote
      xstring
      zref

      # https://wellquite.org/posts/latex_fonts_and_nixos/
      { pkgs = lib.map (pkg: pkg // { tlType = "run"; }) fonts; }
    ]
  );
in
pkgs.mkShellNoCC {
  __structuredAttrs = true;
  strictDeps = true;
  preferLocalBuild = true;

  name = "semantics-and-type-systems";

  packages = with pkgs; [
    (typst.wrapper {
      inherit fonts;
    })
    tinymist
    typstyle
    texenv
    python3Packages.pygments
  ];
}
