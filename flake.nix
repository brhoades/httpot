{
  description = "httpot";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: let
    pkgsFor = system: import nixpkgs {
      inherit system;
    }; in (flake-utils.lib.eachDefaultSystem (system: {
      # envrc
      devShells.default = with (pkgsFor system); mkShell {
        buildInputs = [
          rustup
        ];
      };
    }));
}
