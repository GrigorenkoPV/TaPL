{
  lib,
  stdenvNoCC,
  fetchurl,
  unzip,
}:

stdenvNoCC.mkDerivation {
  pname = "ttf-cmu-sans-serif";
  version = "unstable-2012-08-29";

  src = fetchurl {
    url = "https://fontlibrary.org/assets/downloads/cmu-sans-serif/0da0d92e6bacf5e091c1ae85c4da581d/cmu-sans-serif.zip";
    hash = "sha256:a28376915ebe43e78e580ef9646663f08fe3f782767d1c11aeeb36ee2c712c23";
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    install -m444 -Dt $out/share/fonts/truetype Sans*/*.ttf
    runHook postInstall
  '';

  nativeBuildInputs = [
    unzip
  ];

  meta = {
    homepage = "https://fontlibrary.org/en/font/cmu-sans-serif";
    description = "CMU Sans Serif";
    license = lib.licenses.ofl;
    platforms = lib.platforms.all;
  };
}
