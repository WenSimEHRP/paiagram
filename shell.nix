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
    rustup
    coreutils
    typst
    # it has decent CJK support
    sarasa-gothic
    just
  ];
  shellHook = ''
    export FONTCONFIG_FILE=${fonts}
  '';
}
