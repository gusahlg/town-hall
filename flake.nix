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
            pkgs.vulkan-loader
            pkgs.stdenv.cc.cc.lib
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              clippy
              gcc
              pkg-config
              rustc
              rustfmt
              shaderc
              vulkan-loader
              vulkan-tools
              vulkan-validation-layers
            ];

            shellHook = ''
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:/run/opengl-driver/lib''${LD_LIBRARY_PATH:+:''${LD_LIBRARY_PATH}}"
              export VK_LAYER_PATH="${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d''${VK_LAYER_PATH:+:''${VK_LAYER_PATH}}"

              echo "town-hall shell: Rust, shaderc, and Vulkan tools are ready."
              echo "Run town-hall with: cargo run --release -p town-hall"
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
