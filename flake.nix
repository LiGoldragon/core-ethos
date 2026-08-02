{
  description = "core-ethos — composite whole-Ethos round trips and retained sealed execution data";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = {
      url = "github:LiGoldragon/rust-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-build }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromPkgs pkgs;
        inherit (rust) craneLib toolchain;
        ethosFilter = path: type:
          type == "regular" && pkgs.lib.hasSuffix ".ethos" path;
        src = rust.cleanSource {
          root = ./.;
          extraFilters = [ ethosFilter ];
        };
        commonArguments = { inherit src; strictDeps = true; };
        cargoArtifacts = craneLib.buildDepsOnly (commonArguments // {
          postCheck = ''
            rm -f target/.rustc_info.json
          '';
        });
      in
      {
        packages.default = craneLib.buildPackage (commonArguments // { inherit cargoArtifacts; });
        checks = {
          build = craneLib.cargoBuild (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
          });
          test = craneLib.cargoTest (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
          });
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
            CARGO_BUILD_JOBS = "1";
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt {
            inherit src;
            doInstallCargoArtifacts = false;
          };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            doInstallCargoArtifacts = false;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        devShells.default = pkgs.mkShell {
          name = "core-ethos";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
