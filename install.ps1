#Requires -Version 5.1
<#
.SYNOPSIS
    Windows installer for devcontainer-mcp (via WSL).
.DESCRIPTION
    Installs the devcontainer-mcp Linux binary inside WSL and configures
    Windows-side MCP clients to use the WSL bridge ("command": "wsl").
.EXAMPLE
    irm https://github.com/aniongithub/devcontainer-mcp/releases/latest/download/install.ps1 | iex
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo = "aniongithub/devcontainer-mcp"
$WslBinaryPath = "~/.local/bin/devcontainer-mcp"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Write-Step { param([string]$Message) Write-Host "==> $Message" -ForegroundColor Cyan }
function Write-Ok   { param([string]$Message) Write-Host "  $([char]0x2713) $Message" -ForegroundColor Green }
function Write-Warn { param([string]$Message) Write-Host "  $([char]0x26A0) $Message" -ForegroundColor Yellow }
function Write-Fail { param([string]$Message) Write-Host "  $([char]0x2717) $Message" -ForegroundColor Red }

# ---------------------------------------------------------------------------
# 1. Verify WSL is available
# ---------------------------------------------------------------------------

Write-Step "Checking for WSL..."

try {
    $wslStatus = wsl --status 2>&1
    if ($LASTEXITCODE -ne 0) { throw "WSL returned non-zero exit code" }
    Write-Ok "WSL is available"
} catch {
    Write-Host ""
    Write-Host "Error: WSL (Windows Subsystem for Linux) is required but not found." -ForegroundColor Red
    Write-Host ""
    Write-Host "Install WSL with:  wsl --install" -ForegroundColor Yellow
    Write-Host "Then restart your computer and run this script again."
    Write-Host "More info: https://learn.microsoft.com/en-us/windows/wsl/install"
    exit 1
}

# ---------------------------------------------------------------------------
# 2. Install binary inside WSL (reuse install.sh)
# ---------------------------------------------------------------------------

Write-Step "Installing devcontainer-mcp binary inside WSL..."

$installUrl = "https://raw.githubusercontent.com/$Repo/main/install.sh"
$wslResult = wsl bash -c "curl -fsSL '$installUrl' | bash -s -- --skip-mcp-config" 2>&1
$wslResult | ForEach-Object { Write-Host "    $_" }

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "Error: Binary installation inside WSL failed." -ForegroundColor Red
    Write-Host "Try running manually in WSL: curl -fsSL $installUrl | bash"
    exit 1
}

# Verify the binary works
$versionCheck = wsl bash -lc "$WslBinaryPath --version" 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Ok "Installed: $($versionCheck.Trim())"
} else {
    Write-Warn "Binary installed but could not verify version"
}

# ---------------------------------------------------------------------------
# 3. Install SKILL.md for Windows-side agent discovery
# ---------------------------------------------------------------------------

Write-Step "Installing SKILL.md for agent discovery..."

$skillUrl = "https://raw.githubusercontent.com/$Repo/main/SKILL.md"
$skillDirs = @(
    "$env:USERPROFILE\.copilot\skills\devcontainer-mcp"
    "$env:USERPROFILE\.claude\skills\devcontainer-mcp"
    "$env:USERPROFILE\.agents\skills\devcontainer-mcp"
)

foreach ($dir in $skillDirs) {
    try {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Invoke-RestMethod -Uri $skillUrl -OutFile "$dir\SKILL.md"
        Write-Ok $dir
    } catch {
        Write-Warn "Could not write to $dir"
    }
}

# ---------------------------------------------------------------------------
# 4. Detect backend CLIs available in WSL
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "Backend CLIs detected in WSL (install as needed — MCP server gives helpful errors if missing):"

$backends = @(
    @{ Name = "devpod";       Url = "https://devpod.sh/docs/getting-started/install" }
    @{ Name = "devcontainer"; Url = "npm install -g @devcontainers/cli" }
    @{ Name = "gh";           Url = "https://cli.github.com/" }
)

foreach ($b in $backends) {
    $check = wsl bash -lc "command -v $($b.Name)" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Ok "$($b.Name)"
    } else {
        $label = $b.Name
        if ($label -eq "gh") { $label = "gh (codespaces)" }
        Write-Fail "$label — $($b.Url)"
    }
}

# ---------------------------------------------------------------------------
# 5. Configure Windows-side MCP clients (with WSL bridge)
# ---------------------------------------------------------------------------

Write-Step "Configuring MCP clients..."

$mcpServerEntry = @{
    command = "wsl"
    args    = @($WslBinaryPath, "serve")
}

function Set-McpConfig {
    param(
        [string]$ConfigPath,
        [string]$ClientName
    )

    try {
        if (Test-Path $ConfigPath) {
            $content = Get-Content -Raw $ConfigPath | ConvertFrom-Json
            if (-not $content.mcpServers) {
                $content | Add-Member -NotePropertyName "mcpServers" -NotePropertyValue ([PSCustomObject]@{})
            }
            if ($content.mcpServers.PSObject.Properties.Name -contains "devcontainer-mcp") {
                Write-Ok "$ClientName — already configured"
                return
            }
            $content.mcpServers | Add-Member -NotePropertyName "devcontainer-mcp" -NotePropertyValue ([PSCustomObject]$mcpServerEntry)
            $content | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath -Encoding UTF8
            Write-Ok "$ClientName — added to $ConfigPath"
        } else {
            $dir = Split-Path $ConfigPath -Parent
            if ($dir) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
            $config = [PSCustomObject]@{
                mcpServers = [PSCustomObject]@{
                    "devcontainer-mcp" = [PSCustomObject]$mcpServerEntry
                }
            }
            $config | ConvertTo-Json -Depth 10 | Set-Content $ConfigPath -Encoding UTF8
            Write-Ok "$ClientName — created $ConfigPath"
        }
    } catch {
        Write-Warn "$ClientName — could not update $ConfigPath"
    }
}

# Claude Code
Set-McpConfig "$env:USERPROFILE\.claude.json" "Claude Code"

# GitHub Copilot (if .copilot dir exists)
if (Test-Path "$env:USERPROFILE\.copilot") {
    Set-McpConfig "$env:USERPROFILE\.copilot\mcp-config.json" "GitHub Copilot"
}

# VS Code (if config dir exists)
$vscodeDir = "$env:APPDATA\Code\User"
if (Test-Path $vscodeDir) {
    Set-McpConfig "$vscodeDir\mcp.json" "VS Code"
}

# Cursor (if installed)
if (Test-Path "$env:USERPROFILE\.cursor") {
    Set-McpConfig "$env:USERPROFILE\.cursor\mcp.json" "Cursor"
}

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------

Write-Host ""
Write-Host "Done! devcontainer-mcp is ready to use." -ForegroundColor Green
Write-Host ""
Write-Host "MCP clients are configured to launch the server via WSL:" -ForegroundColor DarkGray
Write-Host "  command: wsl" -ForegroundColor DarkGray
Write-Host "  args:    [$WslBinaryPath, serve]" -ForegroundColor DarkGray
