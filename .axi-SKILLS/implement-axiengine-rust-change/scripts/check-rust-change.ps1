param(
    [string[]]$Path
)

$ErrorActionPreference = 'Stop'

function Get-ChangedPath {
    $tracked = @(& git diff --name-only --diff-filter=ACMRT -- 'AxiEngine/**/*.rs' 'AxiEngine/**/Cargo.toml')
    $staged = @(& git diff --cached --name-only --diff-filter=ACMRT -- 'AxiEngine/**/*.rs' 'AxiEngine/**/Cargo.toml')
    $untracked = @(& git ls-files --others --exclude-standard -- 'AxiEngine/**/*.rs' 'AxiEngine/**/Cargo.toml')
    @($tracked + $staged + $untracked) | Where-Object { $_ } | Sort-Object -Unique
}

function Test-RustPublicDocs {
    param([string]$File)

    $lines = Get-Content -LiteralPath $File -Encoding UTF8
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = $lines[$index]
        $declaration = $line -match '^\s*pub(?:\([^)]*\))?\s+(?:(?:async|const|unsafe)\s+)*(?:struct|enum|trait|fn|mod|type|const|static)\b'
        $field = $line -match '^\s*pub\s+[A-Za-z_][A-Za-z0-9_]*\s*:'
        if (-not ($declaration -or $field)) {
            continue
        }

        $cursor = $index - 1
        while ($cursor -ge 0 -and ($lines[$cursor].Trim() -eq '' -or $lines[$cursor] -match '^\s*#\[')) {
            $cursor--
        }
        if ($cursor -lt 0 -or $lines[$cursor] -notmatch '^\s*///') {
            Write-Warning "${File}:$($index + 1): public declaration may be missing rustdoc"
        }
    }
}

if (-not $Path -or $Path.Count -eq 0) {
    $Path = @(Get-ChangedPath)
}

$files = @($Path | Where-Object { Test-Path -LiteralPath $_ } | Sort-Object -Unique)
if ($files.Count -eq 0) {
    Write-Host 'No changed Rust sources or Cargo manifests were found.'
    exit 0
}

Write-Host "Advisory audit of $($files.Count) changed file(s):"

foreach ($file in $files) {
    Write-Host "- $file"
    if ($file -like '*.rs') {
        Test-RustPublicDocs -File $file
        Select-String -LiteralPath $file -Pattern '\bunsafe\s*(?:\{|fn\b|impl\b|trait\b)' | ForEach-Object {
            Write-Warning "${file}:$($_.LineNumber): inspect authorization and SAFETY documentation for unsafe code"
        }
        Select-String -LiteralPath $file -Pattern '\.(?:unwrap|expect)\s*\(|\bpanic!\s*\(' | ForEach-Object {
            Write-Warning "${file}:$($_.LineNumber): inspect panic-like operation; tests and documented impossibility may be valid"
        }
        Select-String -LiteralPath $file -Pattern '(?i)\b(?:stage\s+[A-Z0-9]|LP-\d+|PR-[A-Z0-9-]+|T\d{3,})\b' | ForEach-Object {
            Write-Warning "${file}:$($_.LineNumber): possible task or stage label in production source"
        }
    }
    elseif ((Split-Path -Leaf $file) -eq 'Cargo.toml') {
        Write-Warning "${file}: manifest changed; inspect dependency, feature, pinning, and downstream feature-forwarding impact"
    }
}

Write-Host 'Audit complete. Warnings are review prompts, not proof of defects.'
