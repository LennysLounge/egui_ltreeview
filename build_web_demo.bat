cargo build --example playground --release --target wasm32-unknown-unknown
wasm-bindgen ./target/wasm32-unknown-unknown/release/examples/playground.wasm --target web --out-dir web_demo --no-typescript