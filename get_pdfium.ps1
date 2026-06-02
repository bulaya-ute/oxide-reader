# get_pdfium.ps1
# Downloads pdfium.dll for Windows from the bblanchon/pdfium-binaries releases.
# Run this once from the project root before building.

$ErrorActionPreference = "Stop"

$arch    = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$url     = "https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-$arch.tgz"
$archive = "pdfium_tmp.tgz"

Write-Host "Downloading PDFium for Windows $arch ..."
Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing

Write-Host "Extracting pdfium.dll ..."

# tar is built into Windows 10 1803+ / Windows 11
tar -xzf $archive bin/pdfium.dll

if (Test-Path "bin\pdfium.dll") {
    Copy-Item "bin\pdfium.dll" "pdfium.dll" -Force
    Remove-Item "bin" -Recurse -Force
} elseif (-not (Test-Path "pdfium.dll")) {
    Write-Host ""
    Write-Host "ERROR: Could not locate pdfium.dll inside the archive." -ForegroundColor Red
    Write-Host "Please download it manually from:"
    Write-Host "  $url" -ForegroundColor Cyan
    Write-Host "Extract pdfium.dll and place it in this directory."
    exit 1
}

Remove-Item $archive -Force

Write-Host ""
Write-Host "SUCCESS: pdfium.dll is ready." -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Build:  cargo build --release"
Write-Host "  2. Run:    .\target\release\pdf-viewer.exe"
Write-Host "  3. Ship:   copy pdf-viewer.exe + pdfium.dll to the same folder."
