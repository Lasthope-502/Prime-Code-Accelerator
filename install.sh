#!/usr/bin/env bash
set -e

echo "🚀 Installing Prime Code Accelerator..."
echo ""

if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Install from https://rustup.rs first."
    exit 1
fi

echo "📦 Building core CLI (Rust)..."
cargo build --release

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp target/release/prime-accelerator "$INSTALL_DIR/accel"
chmod +x "$INSTALL_DIR/accel"
echo "✅ CLI installed as 'accel' in $INSTALL_DIR"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "⚠️  Add this to your ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
fi

if command -v python3 &> /dev/null; then
    echo ""
    read -p "🐍 Build Python fast_ops module? (requires maturin) [y/N]: " build_py
    if [[ "$build_py" == "y" || "$build_py" == "Y" ]]; then
        pip install maturin --quiet
        (cd fast_ops && maturin develop --release)
        echo "✅ fast_ops (Python) installed."
    fi
fi

if command -v node &> /dev/null && command -v npm &> /dev/null; then
    echo ""
    read -p "🟨 Build Node fast_ops_node addon? [y/N]: " build_node
    if [[ "$build_node" == "y" || "$build_node" == "Y" ]]; then
        (cd fast_ops_node && npm install && npm run build)
        echo "✅ fast_ops_node (Node) installed."
    fi
fi

echo ""
echo "🎉 Done! Try: accel run -- python3 your_script.py"