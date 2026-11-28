# SPDX-License-Identifier: AGPL-3.0-or-later
{
  description = "Development shell for office-harness";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # nixpkgs ships a different rustc, so the toolchain comes from this overlay.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSystem = nixpkgs.lib.genAttrs systems;

      shellFor = system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          # The flake follows the file, so the version lives in one place.
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          # The CI "Install Linux system packages for Tauri" step, as nix packages.
          tauriLibraries = with pkgs; [
            webkitgtk_4_1
            libsoup_3
            gtk3
            glib
            cairo
            pango
            gdk-pixbuf
            atk
            librsvg
            openssl
            dbus
            xdotool
            libayatana-appindicator
          ];
        in
        pkgs.mkShell {
          packages = [ rustToolchain pkgs.bun pkgs.nodejs_24 pkgs.just ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux ([ pkgs.pkg-config ] ++ tauriLibraries);

          shellHook = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            # GTK file dialogs abort without the GSettings schemas.
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
          '';
        };
    in
    {
      devShells = forEachSystem (system: { default = shellFor system; });
    };
}
