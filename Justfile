default:
    just --list

build_wasm:
    cd wasm && \
        cargo build --release --target wasm32-unknown-unknown && \
        cp ./target/wasm32-unknown-unknown/release/*.wasm ../src
