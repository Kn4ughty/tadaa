{
  description = "Whats even the point of this description anyway";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (system: 
        let
          pkgs = pkgsFor system;
        in {
          default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "confetti";
            version = "0.1.0";

            src = pkgs.lib.cleanSource ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildInputs = with pkgs; [
              libxkbcommon
              wayland
            ];

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
        in {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.default ];

            nativeBuildInputs = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
            ];
          };
        });
    };
}
