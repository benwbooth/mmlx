{
  description = "mmlx — Rust-embedded music composition DSL and text-first DAW core";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = inputs@{ self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            pkg-config
            # cpal audio backend (`--features audio`) links ALSA on Linux.
            alsa-lib
            # Tracker/chip playback libs for mmlx-track (dlopen at runtime).
            game-music-emu
            libopenmpt
          ];

          shellHook = ''
            export MMLX_GME_LIB="${pkgs.game-music-emu}/lib/libgme.so"
            export MMLX_OPENMPT_LIB="${pkgs.libopenmpt}/lib/libopenmpt.so"
            # nix rustc ships no rust-src; point rust-analyzer at the
            # platform sources so the standard library resolves
            # (same value nixpkgs' own rust-analyzer wrapper uses).
            export RUST_SRC_PATH="${pkgs.rustPlatform.rustLibSrc}"
          '';
        };
      });
    };
}
