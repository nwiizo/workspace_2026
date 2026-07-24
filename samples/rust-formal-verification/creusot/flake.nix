{
  description = "Creusot 0.12.0 with a minimal macOS-compatible prover set";

  inputs = {
    creusot.url = "github:creusot-rs/creusot/v0.12.0";
    nixpkgs.follows = "creusot/nixpkgs";
  };

  outputs =
    {
      creusot,
      nixpkgs,
      self,
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ creusot.overlays.default ];
          };
          rustToolchain = pkgs.creusot.mkRustToolchain [ ];
          why3findConfig = pkgs.writeTextFile {
            name = "why3find.json";
            destination = "/why3find.json";
            text = builtins.readFile ./why3find.json;
          };
          why3Config = pkgs.writeTextFile {
            name = "creusot_why3.conf";
            destination = "/creusot_why3.conf";
            text = builtins.readFile "${creusot}/creusot-install/creusot_why3.conf";
          };
          why3find = pkgs.creusot.why3find.overrideAttrs (previous: {
            nativeBuildInputs =
              (previous.nativeBuildInputs or [ ])
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.sigtool ];
          });
          why3Framework = pkgs.symlinkJoin {
            name = "creusot-why3-minimal";
            paths = [
              pkgs.creusot.alt-ergo-free
              pkgs.creusot.why3
              why3find
              why3findConfig
              why3Config
            ];
            postBuild = "ln -s $out $out/creusot";
          };
          wrapped = pkgs.buildEnv {
            name = "creusot-free-minimal";
            meta.mainProgram = "cargo-creusot";
            paths = [
              rustToolchain
              pkgs.creusot.prelude
              pkgs.creusot.creusot
              why3Framework
            ];
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postBuild = ''
              wrapProgram $out/bin/cargo \
                --add-flag "--config" \
                --add-flag "patch.crates-io.creusot-std.path=\"$out/share/creusot-std\""

              wrapProgram $out/bin/cargo-creusot \
                --set CARGO "$out/bin/cargo" \
                --set CREUSOT_DATA_HOME "$out"
            '';
          };
        in
        {
          default = wrapped;
          creusot-free-minimal = wrapped;
        }
      );
    };
}
