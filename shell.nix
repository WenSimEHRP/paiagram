with import <nixpkgs> { };
let
  fonts = makeFontsConf {
    fontDirectories = [
      sarasa-gothic
    ];
  };
in
mkShell {
  nativeBuildInputs = [
    rustup # wasm support
    coreutils # generic tools
    typst # compiler
    typship # shipping tool
    sarasa-gothic # font for typst
    just # command runner
  ];
  shellHook = ''
    export FONTCONFIG_FILE=${fonts}
  '';
}
