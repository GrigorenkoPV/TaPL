{
  lib,
  stdenvNoCC,
  fetchurl,
  unzip,
}:

stdenvNoCC.mkDerivation {
  pname = "ttf-cmu-typewriter";
  version = "unstable-2012-08-29";

  src = fetchurl {
    url = "https://fontlibrary.org/assets/downloads/cmu-typewriter/b6d120f7029a2f8b50af39054e05f1ef/cmu-typewriter.zip";
    hash = "sha256-HT5zQWnowYjHsT6BGkcVkJDyNPZQykUI92PpIuBXEGY=";
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    install -m444 -Dt $out/share/fonts/truetype Typewriter*/*.ttf
    runHook postInstall
  '';

  nativeBuildInputs = [
    unzip
  ];

  meta = {
    homepage = "https://fontlibrary.org/en/font/cmu-typewriter";
    description = "CMU Typewriter";
    license = lib.licenses.ofl;
    platforms = lib.platforms.all;
  };
}
