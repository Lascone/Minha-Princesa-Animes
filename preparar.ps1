#Requires -Version 5.1
<#
.SYNOPSIS
    Prepara o ambiente completo para desenvolver e buildar o Minha Princesa.

.DESCRIPTION
    Instala (via winget, se necessario):
      - Node.js LTS
      - Rust (rustup)
      - FFmpeg
    Depois executa npm install e valida o toolchain.

.PARAMETER SkipWinget
    Nao tenta instalar dependencias via winget (so valida e instala npm).

.PARAMETER Build
    Apos preparar, executa o build de producao (npm run tauri build).
    Carrega minha-princesa-animes.key automaticamente para assinar o updater.

.PARAMETER PublishSecrets
    Envia as chaves de assinatura para o GitHub Actions via gh secret set.
    Cria TAURI_SIGNING_PRIVATE_KEY e TAURI_SIGNING_PRIVATE_KEY_PASSWORD (vazia se sem senha).

.PARAMETER Dev
    Apos preparar, abre o app em modo desenvolvimento (npm run tauri dev).

.EXAMPLE
    .\preparar.ps1

.EXAMPLE
    .\preparar.ps1 -Dev

.EXAMPLE
    .\preparar.ps1 -Build

.EXAMPLE
    .\preparar.ps1 -PublishSecrets
#>

[CmdletBinding()]
param(
    [switch]$SkipWinget,
    [switch]$Build,
    [switch]$Dev,
    [switch]$PublishSecrets
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = $PSScriptRoot
$TauriDir    = Join-Path $ProjectRoot "src-tauri"

function Write-Step([string]$Message) {
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Ok([string]$Message) {
    Write-Host "    OK  $Message" -ForegroundColor Green
}

function Write-Warn([string]$Message) {
    Write-Host "    !!  $Message" -ForegroundColor Yellow
}

function Write-Err([string]$Message) {
    Write-Host "    ERRO  $Message" -ForegroundColor Red
}

function Get-SigningKeyPath {
    $candidates = @(
        (Join-Path $ProjectRoot "minha-princesa-animes.key"),
        (Join-Path $TauriDir "minha-princesa-animes.key")
    )
    foreach ($path in $candidates) {
        if (Test-Path $path) { return $path }
    }
    return $null
}

function Set-TauriSigningEnv {
  param(
    [string]$Password = ""
  )

  $keyPath = Get-SigningKeyPath
  if (-not $keyPath) {
    Write-Warn "Chave privada nao encontrada. Coloque minha-princesa-animes.key na raiz do projeto."
    return $false
  }

  $keyContent = (Get-Content -Path $keyPath -Raw).Trim()
  if ([string]::IsNullOrWhiteSpace($keyContent)) {
    throw "Arquivo de chave vazio: $keyPath"
  }

  $env:TAURI_SIGNING_PRIVATE_KEY = $keyContent
  $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $Password
  Write-Ok "TAURI_SIGNING_PRIVATE_KEY carregada ($keyPath)"
  return $true
}

function Publish-GitHubSigningSecrets {
  param(
    [string]$Password = ""
  )

  if (-not (Test-CommandExists "gh")) {
    throw "GitHub CLI (gh) nao encontrado. Instale: winget install GitHub.cli"
  }

  $null = gh auth status 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw @"
Nao logado no GitHub. Rode em outro terminal (interativo):

  gh auth login

Escolha: GitHub.com -> HTTPS -> Login with a web browser
Depois rode: .\preparar.ps1 -PublishSecrets
"@
  }

  $keyPath = Get-SigningKeyPath
  if (-not $keyPath) {
    throw "minha-princesa-animes.key nao encontrado na raiz do projeto."
  }

  Write-Step "Enviando secrets para GitHub Actions"
  Write-Host "    Nome obrigatorio: TAURI_SIGNING_PRIVATE_KEY" -ForegroundColor DarkGray
  Write-Host "    (NAO use outro nome, ex: MINHAPRINCESAANIMES)" -ForegroundColor DarkGray

  $keyContent = (Get-Content -Path $keyPath -Raw).Trim()
  $keyContent | gh secret set TAURI_SIGNING_PRIVATE_KEY
  if ($LASTEXITCODE -ne 0) {
    throw "gh secret set TAURI_SIGNING_PRIVATE_KEY falhou. Rode antes: gh auth login"
  }

  if ([string]::IsNullOrEmpty($Password)) {
    # PowerShell: gh secret set -b "" falha; stdin com string vazia funciona
  '' | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  } else {
    gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body $Password
  }
  if ($LASTEXITCODE -ne 0) {
    throw "gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD falhou (codigo $LASTEXITCODE)"
  }

  Write-Ok "Secrets TAURI_SIGNING_PRIVATE_KEY e TAURI_SIGNING_PRIVATE_KEY_PASSWORD configurados"
  Write-Warn "Apague o secret com nome errado (ex: MINHAPRINCESAANIMES) em:"
  Write-Host "    https://github.com/Lascone/Minha-Princesa-Animes/settings/secrets/actions" -ForegroundColor DarkGray
}

function Invoke-NativeQuiet {
    param([scriptblock]$Command)
    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $Command 2>&1 | Out-Null
        return $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
}

function Add-CargoToPath {
    $cargoHome = Join-Path $env:USERPROFILE ".cargo"
    $cargoBin  = Join-Path $cargoHome "bin"

    if (-not (Test-Path $cargoBin)) {
        return $false
    }

    $env:CARGO_HOME  = $cargoHome
    $env:RUSTUP_HOME = $cargoHome

    # Sempre coloca .cargo\bin na frente (npm no Windows pode resetar o PATH)
    $parts = @($env:Path -split ';' | Where-Object { $_ -and ($_.Trim() -ne $cargoBin) })
    $env:Path = (@($cargoBin) + $parts) -join ';'
    return $true
}

function Ensure-CargoPath {
    if (-not (Add-CargoToPath)) {
        throw "cargo nao encontrado em $env:USERPROFILE\.cargo\bin. Reinstale o Rust ou rode sem -SkipWinget."
    }

    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -and $userPath -notlike "*$cargoBin*") {
        [Environment]::SetEnvironmentVariable("Path", "$cargoBin;$userPath", "User")
        Write-Warn "Cargo adicionado ao PATH do Windows (permanente). Reinicie terminais antigos se precisar."
    }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo nao encontrado no PATH da sessao. Feche o terminal, abra um novo e rode .\preparar.ps1 -SkipWinget -Dev"
    }
}

function Refresh-SessionPath {
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath    = [Environment]::GetEnvironmentVariable("Path", "User")

    $parts = @()
    if ($machinePath) { $parts += $machinePath -split ';' }
    if ($userPath)    { $parts += $userPath -split ';' }
    $parts = $parts | Where-Object { $_ } | Select-Object -Unique
    if ($parts.Count -gt 0) {
        $env:Path = ($parts -join ';')
    }

    Add-CargoToPath | Out-Null
}

function Test-CommandExists([string]$Name) {
    if (Get-Command $Name -ErrorAction SilentlyContinue) { return $true }
    if ($Name -eq "npm" -and (Get-Command "npm.cmd" -ErrorAction SilentlyContinue)) { return $true }
    return $false
}

function Invoke-Tool {
    param(
        [Parameter(Mandatory)][string]$Name,
        [string[]]$ToolArgs = @()
    )
    if ($Name -eq "npm") {
        return & npm.cmd @ToolArgs
    }
    if (Get-Command $Name -ErrorAction SilentlyContinue) {
        return & $Name @ToolArgs
    }
    throw "Comando nao encontrado: $Name"
}

function Get-CommandVersion([string]$Name, [string[]]$VersionArgs = @("--version")) {
    if (-not (Test-CommandExists $Name)) { return $null }
    try {
        if ($Name -eq "npm") {
            return (& npm.cmd --version 2>&1 | Select-Object -First 1 | Out-String).Trim()
        }
        $output = Invoke-Tool -Name $Name -ToolArgs $VersionArgs 2>&1 | Select-Object -First 1
        return ($output | Out-String).Trim()
    } catch {
        return $null
    }
}

function Invoke-Npm {
    param(
        [Parameter(Mandatory)][string[]]$NpmArgs
    )
    Refresh-SessionPath
    Ensure-CargoPath
    & npm.cmd @NpmArgs
    if ($LASTEXITCODE -ne 0) {
        throw "npm falhou (codigo $LASTEXITCODE): npm $($NpmArgs -join ' ')"
    }
}

function Invoke-TauriCli {
    param(
        [Parameter(Mandatory)][string[]]$TauriArgs
    )

    Ensure-CargoPath

    $tauriJs = Join-Path $ProjectRoot "node_modules\@tauri-apps\cli\tauri.js"
    if (-not (Test-Path $tauriJs)) {
        throw "CLI do Tauri nao encontrado. Rode .\preparar.ps1 sem -Dev primeiro."
    }

    # Evita 'npm run' — no Windows o npm pode ignorar o PATH ajustado da sessao
    Push-Location $ProjectRoot
    try {
        Write-Host "    cargo: $(Get-Command cargo | Select-Object -ExpandProperty Source)" -ForegroundColor DarkGray
        & node $tauriJs @TauriArgs
        if ($LASTEXITCODE -ne 0) {
            throw "tauri falhou (codigo $LASTEXITCODE): tauri $($TauriArgs -join ' ')"
        }
    } finally {
        Pop-Location
    }
}

function Install-WingetPackage {
    param(
        [Parameter(Mandatory)][string]$Id,
        [Parameter(Mandatory)][string]$Label
    )

    if (-not (Test-CommandExists "winget")) {
        throw "winget nao encontrado. Instale o App Installer da Microsoft Store ou use -SkipWinget."
    }

    $installed = winget list --id $Id --accept-source-agreements 2>$null |
        Select-String -Pattern $Id -Quiet

    if ($installed) {
        Write-Ok "$Label ja instalado ($Id)"
        return
    }

    Write-Host "    Instalando $Label..." -ForegroundColor Gray
    winget install --id $Id `
        --accept-package-agreements `
        --accept-source-agreements `
        --disable-interactivity | Out-Host

    Refresh-SessionPath
    Write-Ok "$Label instalado"
}

function Ensure-Dependency {
    param(
        [Parameter(Mandatory)][string]$Command,
        [Parameter(Mandatory)][string]$Label,
        [string]$WingetId = "",
        [string[]]$VersionArgs = @("--version")
    )

    Refresh-SessionPath

    if (Test-CommandExists $Command) {
        $ver = Get-CommandVersion $Command $VersionArgs
        if ($ver) { Write-Ok "$Label encontrado ($ver)" }
        else      { Write-Ok "$Label encontrado" }
        return
    }

    if ($SkipWinget -or -not $WingetId) {
        throw "$Label nao encontrado ($Command). Instale manualmente ou execute sem -SkipWinget."
    }

    Install-WingetPackage -Id $WingetId -Label $Label
    Refresh-SessionPath

    if (-not (Test-CommandExists $Command)) {
        throw "$Label foi instalado, mas '$Command' ainda nao esta no PATH. Feche e reabra o terminal, depois rode .\preparar.ps1 -SkipWinget"
    }

    $ver = Get-CommandVersion $Command $VersionArgs
    if ($ver) { Write-Ok "$Label pronto ($ver)" }
    else      { Write-Ok "$Label pronto" }
}

function Ensure-RustToolchain {
    Refresh-SessionPath

    if (-not (Test-CommandExists "rustup")) {
        if ($SkipWinget) {
            throw "Rust nao encontrado. Instale em https://rustup.rs ou execute sem -SkipWinget."
        }
        Install-WingetPackage -Id "Rustlang.Rustup" -Label "Rust (rustup)"
        Refresh-SessionPath
    }

    if (-not (Test-CommandExists "rustc")) {
        throw "rustup instalado, mas rustc nao esta no PATH. Reabra o terminal e tente novamente."
    }

    Write-Host "    Configurando toolchain stable..." -ForegroundColor Gray
    Invoke-NativeQuiet { rustup default stable } | Out-Null

    $rustcVer = Get-CommandVersion "rustc" @("--version")
    $cargoVer = Get-CommandVersion "cargo" @("--version")
    Write-Ok "Rust $rustcVer"
    Write-Ok "Cargo $cargoVer"
}

function Install-NpmDependencies {
    Write-Step "Instalando dependencias npm"
    Push-Location $ProjectRoot
    try {
        if (-not (Test-Path (Join-Path $ProjectRoot "package.json"))) {
            throw "package.json nao encontrado em $ProjectRoot"
        }
        Invoke-Npm -NpmArgs @("install")
        Write-Ok "Dependencias npm instaladas"
    } finally {
        Pop-Location
    }
}

function Fetch-RustDependencies {
    Write-Step "Baixando dependencias Rust (primeira vez pode demorar)"
    Ensure-CargoPath
    Push-Location $TauriDir
    try {
        cargo fetch
        if ($LASTEXITCODE -ne 0) {
            throw "cargo fetch falhou (codigo $LASTEXITCODE)"
        }
        Write-Ok "Dependencias Rust baixadas"
    } finally {
        Pop-Location
    }
}

function Test-RustBuild {
    Write-Step "Validando compilacao Rust (debug)"
    Ensure-CargoPath
    Push-Location $TauriDir
    try {
        cargo build
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build falhou (codigo $LASTEXITCODE)"
        }
        Write-Ok "Backend Rust compilado com sucesso"
    } finally {
        Pop-Location
    }
}

function Show-Summary {
    Write-Step "Resumo do ambiente"
    Refresh-SessionPath

    $checks = @(
        @{ Label = "Node.js";  Cmd = "node";   Args = @("--version") },
        @{ Label = "npm";      Cmd = "npm";    Args = @("--version") },
        @{ Label = "Rust";     Cmd = "rustc";  Args = @("--version") },
        @{ Label = "Cargo";    Cmd = "cargo";  Args = @("--version") },
        @{ Label = "FFmpeg";   Cmd = "ffmpeg"; Args = @("-version") }
    )

    foreach ($check in $checks) {
        $ver = Get-CommandVersion $check.Cmd $check.Args
        if ($ver) {
            Write-Ok ("{0,-8} {1}" -f $check.Label, $ver)
        } else {
            Write-Warn ("{0,-8} NAO ENCONTRADO" -f $check.Label)
        }
    }

    Write-Host ""
    Write-Host "Proximos passos:" -ForegroundColor White
    Write-Host "  Desenvolvimento : npm run tauri dev" -ForegroundColor Gray
    Write-Host "  Build instalador: .\preparar.ps1 -Build" -ForegroundColor Gray
    Write-Host "  Secrets GitHub : .\preparar.ps1 -PublishSecrets" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Assinatura updater (local + CI):" -ForegroundColor White
    Write-Host "  Arquivo: minha-princesa-animes.key (na raiz, NUNCA commitar)" -ForegroundColor Gray
    Write-Host "  GitHub secret: TAURI_SIGNING_PRIVATE_KEY" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Instaladores gerados em:" -ForegroundColor White
    Write-Host "  src-tauri\target\release\bundle\" -ForegroundColor Gray
}

# --- Main ---

Write-Host ""
Write-Host "  Minha Princesa - Preparacao do ambiente" -ForegroundColor Magenta
Write-Host "  $ProjectRoot" -ForegroundColor DarkGray

if (-not (Test-Path $TauriDir)) {
    throw "Pasta src-tauri nao encontrada. Execute este script na raiz do projeto Minha Princesa."
}

Write-Step "Verificando dependencias do sistema"

Ensure-Dependency -Command "node"   -Label "Node.js" -WingetId "OpenJS.NodeJS.LTS"
Ensure-Dependency -Command "npm"   -Label "npm"     -WingetId ""
Ensure-RustToolchain
Ensure-Dependency -Command "ffmpeg" -Label "FFmpeg" -WingetId "Gyan.FFmpeg" -VersionArgs @("-version")
& (Join-Path $ProjectRoot "scripts\stage-ffmpeg.ps1")

Install-NpmDependencies
Fetch-RustDependencies
Test-RustBuild

Show-Summary

if ($PublishSecrets) {
    Publish-GitHubSigningSecrets
}

if ($Build) {
    Write-Step "Preparando FFmpeg para o instalador"
    & (Join-Path $ProjectRoot "scripts\stage-ffmpeg.ps1")
    Write-Step "Carregando chave de assinatura do updater"
    if (-not (Set-TauriSigningEnv)) {
        throw @"
minha-princesa-animes.key obrigatoria para o build (createUpdaterArtifacts esta ativo).
Coloque o arquivo na raiz do projeto e rode novamente: .\preparar.ps1 -Build
"@
    }
    Write-Step "Gerando instalador de producao"
    try {
        Invoke-TauriCli -TauriArgs @("build")
        Write-Ok "Build concluido"
    } catch {
        throw
    }
}

if ($Dev) {
    Write-Step "Iniciando modo desenvolvimento"
    Invoke-TauriCli -TauriArgs @("dev")
}

Write-Host ""
Write-Host "Preparacao concluida!" -ForegroundColor Green
Write-Host ""
