cargo build --example playground --release --target wasm32-unknown-unknown
wasm-bindgen ./target/wasm32-unknown-unknown/release/examples/playground.wasm --no-modules --no-typescript --out-dir web_demo