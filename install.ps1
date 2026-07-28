Write-Host "🚀 Installing Prime Code Accelerator..." -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust/Cargo not found. Install from https://rustup.rs first." -ForegroundColor Red
    exit 1
}

Write-Host "📦 Building core CLI..."
cargo build --release

$installDir = "$env:USERPROFILE\.accel\bin"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item "target\release\prime-accelerator.exe" "$installDir\accel.exe" -Force

Write-Host "✅ CLI installed at $installDir\accel.exe" -ForegroundColor Green
Write-Host "⚠️  Add '$installDir' to your PATH environment variable." -ForegroundColor Yellow

$buildPy = Read-Host "🐍 Build Python fast_ops module? (y/N)"
if ($buildPy -eq "y") {
    pip install maturin
    Push-Location fast_ops
    maturin develop --release
    Pop-Location
}

$buildNode = Read-Host "🟨 Build Node fast_ops_node addon? (y/N)"
if ($buildNode -eq "y") {
    Push-Location fast_ops_node
    npm install
    npm run build
    Pop-Location
}

Write-Host "🎉 Done! Try: accel run -- python script.py" -ForegroundColor Cyan