param(
    [string]$RepoRoot = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\..\.."))
} else {
    $RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
}

$docsRoot = Join-Path $RepoRoot "docs\engine"
$indexPath = Join-Path $docsRoot "INDEX.md"

if (-not (Test-Path -LiteralPath $docsRoot -PathType Container)) {
    Write-Error "AxiEngine docs root not found: $docsRoot"
}

$specs = @(
    foreach ($layerDir in Get-ChildItem -LiteralPath $docsRoot -Directory) {
        if ($layerDir.Name -like "spec_L*") {
            Get-ChildItem -LiteralPath $layerDir.FullName -Filter "*_spec.md" -File
        }
    }
)
if ($specs.Count -eq 0) {
    Write-Error "No crate specifications found under $docsRoot\spec_L*"
}

$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()
$versionsByRelativePath = @{}
$invariantOwners = @{}

foreach ($spec in $specs) {
    $content = Get-Content -LiteralPath $spec.FullName -Raw -Encoding UTF8
    $relative = $spec.FullName.Substring($docsRoot.Length).TrimStart("\").Replace("\", "/")
    $expectedTitle = "spec_" + ($spec.BaseName -replace "_spec$", "")

    if ($content -notmatch "(?m)^#\s+$([regex]::Escape($expectedTitle))\s*$") {
        $errors.Add("${relative}: expected title '# $expectedTitle'.")
    }

    if ($content -match "(?m)^>\s*[^:\r\n]+:\s*v?([0-9]+(?:\.[0-9]+)*)") {
        $versionsByRelativePath[$relative] = $Matches[1]
    } else {
        $errors.Add("${relative}: missing or invalid specification version header.")
    }

    if ($content -notmatch "(?m)^>\s*[^:\r\n]+:\s*[0-9]{4}-[0-9]{2}-[0-9]{2}") {
        $errors.Add("${relative}: missing ISO date header.")
    }

    foreach ($required in @(
        @{ Name = "identification"; Pattern = "(?m)^##\s+\u00A71\." },
        @{ Name = "environment"; Pattern = "(?m)^##\s+\u00A72\." },
        @{ Name = "test matrix"; Pattern = "(?im)^##\s+.*(Golden Tests|\u0422\u0435\u0441\u0442)" }
    )) {
        if ($content -notmatch $required.Pattern) {
            $errors.Add("${relative}: missing required $($required.Name) section.")
        }
    }

    if ($content -notmatch "(?im)^##\s+.*(Ownership Boundaries|\u0413\u0440\u0430\u043D\u0438\u0446)") {
        $warnings.Add("${relative}: no explicit ownership-boundaries section.")
    }
    if ($content -notmatch "(?im)^##\s+.*\u0418\u043D\u0432\u0430\u0440\u0438\u0430\u043D\u0442") {
        $warnings.Add("${relative}: no explicit required-invariants section.")
    }

    $declarations = [regex]::Matches($content, "(?m)^\s*-\s+\*\*(INV-[A-Z0-9-]+)\*\*:")
    foreach ($declaration in $declarations) {
        $id = $declaration.Groups[1].Value
        if ($invariantOwners.ContainsKey($id)) {
            $errors.Add("Duplicate invariant declaration $id in $relative and $($invariantOwners[$id]).")
        } else {
            $invariantOwners[$id] = $relative
        }
    }
}

if (Test-Path -LiteralPath $indexPath -PathType Leaf) {
    $indexContent = Get-Content -LiteralPath $indexPath -Raw -Encoding UTF8
    $links = [regex]::Matches($indexContent, "\[[^\]]+\]\((spec_L[0-9]+/[A-Za-z0-9_-]+_spec\.md)\)[^\r\n]*?\bv([0-9]+(?:\.[0-9]+)*)")
    foreach ($link in $links) {
        $relative = $link.Groups[1].Value
        $indexVersion = $link.Groups[2].Value
        if (-not $versionsByRelativePath.ContainsKey($relative)) {
            $errors.Add("INDEX.md references missing specification: $relative.")
        } elseif ($versionsByRelativePath[$relative] -ne $indexVersion) {
            $errors.Add("Version drift for ${relative}: spec v$($versionsByRelativePath[$relative]), INDEX.md v$indexVersion.")
        }
    }
} else {
    $errors.Add("Missing docs/engine/INDEX.md.")
}

foreach ($warning in $warnings) {
    Write-Warning $warning
}
foreach ($errorMessage in $errors) {
    Write-Host "ERROR: $errorMessage" -ForegroundColor Red
}

Write-Host "Checked $($specs.Count) specifications and $($invariantOwners.Count) invariant declarations."
Write-Host "Warnings: $($warnings.Count); Errors: $($errors.Count)."

if ($errors.Count -gt 0) {
    exit 1
}
