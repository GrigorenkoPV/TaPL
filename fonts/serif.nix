{
  lib,
  stdenvNoCC,
  fetchurl,
  unzip,
}:

stdenvNoCC.mkDerivation {
  pname = "ttf-cmu-serif";
  version = "unstable-2012-08-29";

  src = fetchurl {
    url = "https://fontlibrary.org/assets/downloads/cmu-serif/59c4efc3fa08079ca44060ca808c8f58/cmu-serif.zip";
    hash = "sha256:1e9faf3b1a054cd94507469eeda2a3acab1774bb4768b1585e2bdf4558e3c633";
  };

  installPhase = ''
    runHook preInstall
    install -m444 -Dt $out/share/fonts/truetype *.ttf
    runHook postInstall
  '';

  nativeBuildInputs = [
    unzip
  ];

  meta = {
    homepage = "https://fontlibrary.org/en/font/cmu-serif";
    description = "CMU Serif";
    license = lib.licenses.ofl;
    platforms = lib.platforms.all;
  };
}
