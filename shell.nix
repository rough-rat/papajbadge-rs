{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/1d53b40cffa7e78db147341de4f9bf2da2bfea9e.tar.gz") {}
}:

pkgs.mkShell {
  name = "papaj";
  buildInputs = with pkgs; [
    wchisp wlink rustup
  ];
}
