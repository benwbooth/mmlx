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
            nodejs # vscode-mmlx deps (`npm install` in the shellHook below)
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
            # Auto-install the project VSCode extension (./vscode-mmlx) so it
            # loads whenever this project is opened. Idempotent: links only
            # when missing and installs npm deps only once.
            if [ -f "$PWD/vscode-mmlx/package.json" ] && [ -n "${HOME:-}" ]; then
              for _mmlx_vscode_dir in "$HOME/.vscode/extensions" "$HOME/.vscode-oss/extensions"; do
                if [ -d "$_mmlx_vscode_dir" ] && [ ! -e "$_mmlx_vscode_dir/mmlx" ] && [ ! -L "$_mmlx_vscode_dir/mmlx" ]; then
                  ln -s "$PWD/vscode-mmlx" "$_mmlx_vscode_dir/mmlx"
                fi
              done
              unset _mmlx_vscode_dir
              if [ ! -d "$PWD/vscode-mmlx/node_modules" ]; then
                (cd "$PWD/vscode-mmlx" && npm install --no-audit --no-fund >/dev/null 2>&1 || echo "mmlx: vscode extension deps need manual 'cd vscode-mmlx && npm install'")
              fi
            fi
          '';
        };
      });
    };
}
