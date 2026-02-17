{
  stdenvNoCC,
  fetchurl,
}:

stdenvNoCC.mkDerivation {
  pname = "ttf-menlo";
  version = "14.0d1e1";

  srcs = [
    (fetchurl {
      name = "Menlo.ttf";
      url = "https://ofont.ru/index.php?act=download&font_id=5896";
      hash = "sha256-SKWE3cwAJOm3DbsS7vv/mlnRAxoo84QXIZDd28O7vnk=";
    })
    (fetchurl {
      name = "MenloBold.ttf";
      url = "https://ofont.ru/index.php?act=download&font_id=5971";
      hash = "sha256-5GH2n4NdH6XKTyezfj0VoZXlsDu19IaA7RAsZp0qEmw=";
    })

    (fetchurl {
      name = "MenloItalic.ttf";
      url = "https://ofont.ru/index.php?act=download&font_id=5977";
      hash = "sha256-5CFEye0f6IpCFYC1c0bm+NbtW/+XcrC08e+f8LjZVGw=";
    })
  ];

  dontUnpack = true;

  installPhase = ''
    runHook preInstall
    install -m444 -Dt $out/share/fonts/truetype $srcs
    runHook postInstall
  '';
}
