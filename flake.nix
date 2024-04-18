{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";  
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
  in {

    devShells."${system}".default = let
      pkgs = import nixpkgs {
        inherit system;
      };
    in pkgs.mkShell {
      packages = with pkgs; [
        #gcc
        clang
        gnumake
        cmake
	      python3
	      pkg-config
	      jemalloc
	      linuxPackages_latest.perf
	      hyperfine
        glibc_memusage
        valgrind
      ];
    };
  };
}
