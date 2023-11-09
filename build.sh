cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir ./dist --target web ./target/wasm32-unknown-unknown/release/gradient.wasm
wasm-opt -Os -o ./dist/gradient_bg.wasm ./dist/gradient_bg.wasm
cp -r ./assets ./dist/assets
cp ./index.html ./dist/index.html
