{
  lib,
  rustPlatform,
}:
let
  manifest = lib.fromTOML (lib.readFile ./Cargo.toml);
  pname = manifest.package.name;
  version = manifest.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version;

  __structuredAttrs = true;

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = {
    platforms = lib.platforms.all;
    mainProgram = pname;
  };
}
