{
  description = "town-hall development and runtime environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };

          # ash loads Vulkan dynamically at runtime, so both the Nix-provided
          # loader and the host's hardware-driver directory must be visible.
          # /run/opengl-driver is populated by the NixOS graphics configuration
          # and contains the NVIDIA/AMD/Intel implementation selected by the OS.
          runtimeLibraryPath = pkgs.lib.makeLibraryPath [
            pkgs.stdenv.cc.cc.lib
            pkgs.libglvnd
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxkbcommon
            pkgs.libxrandr
            pkgs.vulkan-loader
            pkgs.wayland
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              binaryen
              cargo
              clippy
              gcc
              libglvnd
              libx11
              libxcursor
              libxi
              libxkbcommon
              libxrandr
              lld
              pkg-config
              python3
              rustc
              rustfmt
              shaderc
              vulkan-loader
              vulkan-tools
              vulkan-validation-layers
              wasm-pack
              wayland
            ];

            shellHook = ''
              export TOWN_HALL_DEV_SHELL=1
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:/run/opengl-driver/lib''${LD_LIBRARY_PATH:+:''${LD_LIBRARY_PATH}}"
              export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d''${VK_LAYER_PATH:+:''${VK_LAYER_PATH}}"

              echo "town-hall shell: Rust, eframe, WebAssembly, and Vulkan tools are ready."
              echo "Native client: ./apps/town-hall-client/run.sh"
              echo "CLI:           ./apps/town-hall-cli/run.sh 100"
              echo "Web client:    ./apps/town-hall-client/web/run.sh"
            '';
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.nixfmt
      );
    };
}
