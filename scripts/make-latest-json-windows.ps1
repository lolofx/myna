# Generates and self-validates the Windows-only `latest.json` update manifest
# consumed by the Tauri updater plugin.
#
# Windows counterpart of scripts/make-latest-json.sh, which cannot serve a
# Windows-only fork: it requires the macOS updater artifact and uses `lipo`.
# The manifest shape (version, notes, pub_date, platforms{url, signature}) is
# fixed by the updater plugin; the only platform emitted is windows-x86_64.
#
# The version comes from app/src-tauri/tauri.conf.json, never from the tag.
# The tag is checked against it, so a mismatch fails loudly instead of
# publishing a manifest that advertises a version nobody built.
#
# Usage:
#   ./scripts/make-latest-json-windows.ps1 -Installer <path-setup.exe> `
#       -Tag v0.7.0-beta.1 -Repo <owner>/<name> [-Notes <text>] [-Out <path>]
#
# The installer's sibling <path>.sig supplies the signature.
param(
  [Parameter(Mandatory = $true)][string]$Installer,
  [string]$Tag = $env:GITHUB_REF_NAME,
  [string]$Repo = $env:GITHUB_REPOSITORY,
  [string]$Notes = '',
  [string]$Out = ''
)

$ErrorActionPreference = 'Stop'

function Log($msg)  { Write-Host "`n==> $msg" }
function Fail($msg) { Write-Error "`nFAIL: $msg"; exit 1 }

$RepoRoot  = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$TauriConf = Join-Path $RepoRoot 'app\src-tauri\tauri.conf.json'

if (-not (Test-Path $Installer)) { Fail "Installer not found: $Installer" }
$SigPath = "$Installer.sig"
if (-not (Test-Path $SigPath)) { Fail "Signature file not found: $SigPath" }
if ([string]::IsNullOrWhiteSpace($Tag))  { Fail 'No tag: pass -Tag or set GITHUB_REF_NAME' }
if ([string]::IsNullOrWhiteSpace($Repo)) { Fail 'No repository: pass -Repo <owner>/<name> or set GITHUB_REPOSITORY' }
if ($Repo -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') { Fail "Repository '$Repo' is not of the form owner/name" }
if (-not $Out) { $Out = Join-Path (Split-Path -Parent (Resolve-Path $Installer)) 'latest.json' }

# --- 1. Version: single source of truth is tauri.conf.json ------------------
$version = (Get-Content $TauriConf -Raw | ConvertFrom-Json).version
if ([string]::IsNullOrWhiteSpace($version)) { Fail "version read from $TauriConf is empty" }
Log "Version (from tauri.conf.json): $version"

# --- 2. Tag/version drift guard ---------------------------------------------
if ($Tag -ne "v$version") { Fail "Tag/version drift: tag is '$Tag' but tauri.conf.json version is '$version' (expected tag 'v$version')" }
Log "Tag matches version: $Tag"

# --- 3. Signature: whole content of the .sig file, verbatim -----------------
$signature = (Get-Content $SigPath -Raw).Trim()
if ([string]::IsNullOrEmpty($signature)) { Fail "$SigPath is empty: cannot publish an unsigned updater manifest" }

# --- 4. Notes and date ------------------------------------------------------
if (-not $Notes) { $Notes = "See the release notes at https://github.com/$Repo/releases/tag/$Tag" }
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")

# --- 5. Emit and self-validate ----------------------------------------------
$fileName = Split-Path -Leaf $Installer
$manifest = [ordered]@{
  version   = $version
  notes     = $Notes
  pub_date  = $pubDate
  platforms = [ordered]@{
    'windows-x86_64' = [ordered]@{
      url       = "https://github.com/$Repo/releases/download/$Tag/$([uri]::EscapeDataString($fileName))"
      signature = $signature
    }
  }
}
$json = $manifest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText($Out, $json + "`n", (New-Object Text.UTF8Encoding($false)))

$raw = Get-Content $Out -Raw
$check = $raw | ConvertFrom-Json
if ($check.version -ne $version) { Fail "$Out does not round-trip" }
# Checked on the raw text: ConvertFrom-Json turns the date into an object.
if ($raw -notmatch '"pub_date":\s*"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z"') { Fail 'pub_date is not RFC 3339' }
if (-not $check.platforms.'windows-x86_64'.url.StartsWith("https://github.com/$Repo/releases/download/$Tag/")) { Fail 'installer url is not under the release' }

Log "Wrote $Out"
Write-Host "  Version:  $version"
Write-Host "  Tag:      $Tag"
Write-Host "  URL:      $($check.platforms.'windows-x86_64'.url)"
Write-Host "  pub_date: $pubDate"
