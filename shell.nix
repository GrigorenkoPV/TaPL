{
  pkgs ? import <nixpkgs> { },
}:
let
  inherit (pkgs) lib texlive;

  fonts = with pkgs; [
    (callPackage ./fonts/serif.nix { })
    (callPackage ./fonts/sans-serif.nix { })
    (callPackage ./fonts/typewriter.nix { })
  ];

  texenv = texlive.combine {
    inherit (texlive)
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
      unicode-math
      upquote
      xstring
      zref
      ;

    # https://wellquite.org/posts/latex_fonts_and_nixos/
    latexFonts.pkgs = lib.map (pkg: pkg // { tlType = "run"; }) fonts;
  };
in
pkgs.mkShellNoCC {
  preferLocalBuild = true;
  allowSubstitutes = false;

  name = "semantics-and-type-systems";

  packages = with pkgs; [
    typst
    tinymist
    typstyle
    texenv
    python3Packages.pygments
  ];

  env.FONTCONFIG_FILE = pkgs.makeFontsConf {
    fontDirectories = fonts;
  };
}
