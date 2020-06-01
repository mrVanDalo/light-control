{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {

  buildInputs = with pkgs; [
    rustc
    cargo
    rustfmt

    mosquitto
    cmake
    openssl

    (pkgs.writers.writeBashBin "reformat" ''
      for file in `find ${toString ./.} -type f | egrep "\.rs$"`
      do
        ${pkgs.rustfmt}/bin/rustfmt "$file"
      done

      for file in `find ${toString ./.} -type f | egrep "\.nix$"`
      do
        ${pkgs.nixfmt}/bin/nixfmt "$file"
      done

    '')
  ];

}
