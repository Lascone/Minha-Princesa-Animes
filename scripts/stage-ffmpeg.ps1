#Requires -Version 5.1
<#
.SYNOPSIS
    Copia o FFmpeg do sistema para src-tauri/binaries (empacotamento Tauri).
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$BinDir = Join-Path $ProjectRoot "src-tauri\binaries"
$Target = Join-Path $BinDir "ffmpeg-x86_64-pc-windows-msvc.exe"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

if (Test-Path $Target) {
    $size = (Get-Item $Target).Length
    if ($size -gt 1MB) {
        Write-Host "FFmpeg já preparado em binaries ($([math]::Round($size / 1MB, 1)) MB)"
        exit 0
    }
}

function Find-FfmpegExe {
    $cmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { return $cmd.Source }

    $wingetLink = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links\ffmpeg.exe"
    if (Test-Path $wingetLink) { return $wingetLink }

    $packages = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages"
    if (Test-Path $packages) {
        $found = Get-ChildItem $packages -Recurse -Filter "ffmpeg.exe" -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }

    return $null
}

$source = Find-FfmpegExe
if (-not $source) {
    Write-Host "FFmpeg não encontrado no sistema. Baixando para o instalador..."
    $zip = Join-Path $env:TEMP "minha_princesa_ffmpeg_build.zip"
    $extract = Join-Path $env:TEMP "minha_princesa_ffmpeg_extract"
    if (Test-Path $extract) { Remove-Item $extract -Recurse -Force }
    Invoke-WebRequest -Uri "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip" -OutFile $zip
    Expand-Archive -Path $zip -DestinationPath $extract -Force
    $found = Get-ChildItem $extract -Recurse -Filter "ffmpeg.exe" -ErrorAction SilentlyContinue |
        Where-Object { $_.DirectoryName -match '\\bin$' } |
        Select-Object -First 1
    if (-not $found) { throw "ffmpeg.exe não encontrado no pacote baixado" }
    $source = $found.FullName
    Remove-Item $zip -Force -ErrorAction SilentlyContinue
}

Copy-Item $source $Target -Force
Write-Host "FFmpeg copiado para $Target"
