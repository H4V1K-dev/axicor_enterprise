param(
    [string]$RepoRoot = "",
    [string]$ProgramSlug = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\..\.."))
} else {
    $RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
}

$researchRoot = Join-Path $RepoRoot "docs\engine\research"
$archiveRoot = Join-Path $researchRoot "archive"
$activeRoot = Join-Path $archiveRoot "_active"
$statusPath = Join-Path $researchRoot "current_biocalibration_status.md"
$scanRoot = $archiveRoot
$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()

if (-not [string]::IsNullOrWhiteSpace($ProgramSlug)) {
    if ($ProgramSlug -notmatch '^[a-z0-9][a-z0-9_]*$') {
        Write-Error "ProgramSlug must be a lowercase ASCII research slug."
    }
    $scanRoot = Join-Path $activeRoot $ProgramSlug
    if (-not (Test-Path -LiteralPath $scanRoot -PathType Container)) {
        Write-Error "Active research program not found: $scanRoot"
    }
}

function Get-ResearchRelativePath([string]$FullPath) {
    return $FullPath.Substring($researchRoot.Length).TrimStart("\").Replace("\", "/")
}

if (-not (Test-Path -LiteralPath $researchRoot -PathType Container)) {
    Write-Error "Research root not found: $researchRoot"
}
if (-not (Test-Path -LiteralPath (Join-Path $researchRoot "RULES.md") -PathType Leaf)) {
    $errors.Add("Missing docs/engine/research/RULES.md.")
}
if (-not (Test-Path -LiteralPath $statusPath -PathType Leaf)) {
    $errors.Add("Missing current_biocalibration_status.md.")
}
if (-not (Test-Path -LiteralPath $archiveRoot -PathType Container)) {
    $errors.Add("Missing research archive directory.")
}

$allowedRootFiles = @("RULES.md", "current_biocalibration_status.md")
foreach ($file in Get-ChildItem -LiteralPath $researchRoot -File) {
    if ($allowedRootFiles -notcontains $file.Name) {
        $errors.Add("Loose file in research root: $($file.Name).")
    }
}

if ([string]::IsNullOrWhiteSpace($ProgramSlug) -and (Test-Path -LiteralPath $archiveRoot -PathType Container)) {
    foreach ($dir in Get-ChildItem -LiteralPath $archiveRoot -Directory) {
        if ($dir.Name -ne "_active" -and $dir.Name -notmatch "^[0-9]{4}-[0-9]{2}-[0-9]{2}_[a-z0-9][a-z0-9_]*$") {
            $warnings.Add("Non-dated or noncanonical archive directory: $($dir.Name).")
        }
    }
}

$readmes = @(Get-ChildItem -LiteralPath $scanRoot -Recurse -Filter "README.md" -File -ErrorAction SilentlyContinue)
foreach ($readme in $readmes) {
    $content = Get-Content -LiteralPath $readme.FullName -Raw -Encoding UTF8
    $relative = Get-ResearchRelativePath $readme.FullName
    $isActive = $readme.FullName.StartsWith($activeRoot, [System.StringComparison]::OrdinalIgnoreCase)

    if ($content -notmatch "(?im)^Status:\s*.+$") {
        $warnings.Add("${relative}: missing Status metadata.")
    }
    if ($isActive -and $content -notmatch "(?im)^Started:\s*[0-9]{4}-[0-9]{2}-[0-9]{2}\s*$") {
        $warnings.Add("${relative}: active program missing ISO Started date.")
    }
    if ($isActive -and $content -match "(?im)^Status:\s*.*(archived|finished|superseded|abandoned)\b") {
        $errors.Add("${relative}: terminal status remains under archive/_active.")
    }

    if ($isActive -and $content -match "(?i)rejected") {
        $programDir = $readme.Directory.FullName
        foreach ($report in Get-ChildItem -LiteralPath $programDir -Recurse -Filter "*.md" -File) {
            if ($report.FullName -eq $readme.FullName) {
                continue
            }
            $head = (Get-Content -LiteralPath $report.FullName -Encoding UTF8 | Select-Object -First 12) -join "`n"
            if ($head -match "(?i)planned") {
                $reportRelative = Get-ResearchRelativePath $report.FullName
                $warnings.Add("${reportRelative}: report header still says planned while active README contains a rejected verdict.")
            }
        }
    }

    if ($isActive) {
        $programDir = $readme.Directory.FullName
        $phaseRows = [regex]::Matches($content, '(?im)^\|\s*\*\*?[A-Z][0-9]+\*\*?\s*\|').Count
        if ($phaseRows -ge 2 -and -not (Test-Path -LiteralPath (Join-Path $programDir 'narrative.md') -PathType Leaf)) {
            $warnings.Add("${relative}: multi-gate program has $phaseRows phase rows but no living narrative.md.")
        }

        $programMarkdown = @(Get-ChildItem -LiteralPath $programDir -Recurse -Filter '*.md' -File -ErrorAction SilentlyContinue)
        $programText = ($programMarkdown | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw -Encoding UTF8 }) -join "`n"
        if ($programText -match '(?i)\b(?:runner|cargo\s+test|rust\s+test)\b' -and $programText -notmatch '(?im)^\s*(?:cargo\s+(?:test|run)|powershell\s+.+|python\s+.+)') {
            $warnings.Add("${relative}: runner is discussed but no exact reproduction command was found in the active package.")
        }

        foreach ($report in $programMarkdown) {
            if ($report.Name -eq 'README.md' -or $report.Name -eq 'narrative.md') {
                continue
            }
            $reportContent = Get-Content -LiteralPath $report.FullName -Raw -Encoding UTF8
            foreach ($verdictMatch in [regex]::Matches($reportContent, '(?i)Hypothesis\s+\*\*([A-Z][0-9]+)\s+is\s+(SUPPORTED|WEAKENED|REJECTED|INCONCLUSIVE|INVALID)\*\*')) {
                $gate = $verdictMatch.Groups[1].Value
                if ($content -match "(?im)^\|[^`n]*\*\*$gate\*\*[^`n]*\|\s*(Running|Planned)\s*\|\s*(Pending|-)\s*\|") {
                    $warnings.Add("${relative}: gate $gate has a terminal verdict in a report but remains $($Matches[1])/$($Matches[2]) in README.")
                }
            }
        }
    }
}

$markdownFiles = @(Get-ChildItem -LiteralPath $scanRoot -Recurse -Filter "*.md" -File)
if ([string]::IsNullOrWhiteSpace($ProgramSlug)) {
    $markdownFiles += @(Get-ChildItem -LiteralPath $researchRoot -File -Filter '*.md')
}
$linkCount = 0
foreach ($markdown in $markdownFiles) {
    $content = Get-Content -LiteralPath $markdown.FullName -Raw -Encoding UTF8
    foreach ($absoluteMatch in [regex]::Matches($content, '(?im)(?:file:///[^\s)]+|\b[A-Za-z]:[\\/][^\s)`]+)')) {
        $sourceRelative = Get-ResearchRelativePath $markdown.FullName
        $warnings.Add("${sourceRelative}: machine-specific absolute path: $($absoluteMatch.Value).")
    }
    $matches = [regex]::Matches($content, "\[[^\]]*\]\(([^)]+)\)")
    foreach ($match in $matches) {
        $rawTarget = $match.Groups[1].Value.Trim()
        $target = ($rawTarget -split "#", 2)[0].Trim().Trim("<", ">")
        if ([string]::IsNullOrWhiteSpace($target) -or $target -match "^(https?://|mailto:)") {
            continue
        }
        $linkCount++
        try {
            $resolved = [System.IO.Path]::GetFullPath((Join-Path $markdown.Directory.FullName $target))
        } catch {
            $sourceRelative = Get-ResearchRelativePath $markdown.FullName
            $errors.Add("${sourceRelative}: invalid local link target: $rawTarget.")
            continue
        }
        if (-not (Test-Path -LiteralPath $resolved)) {
            $sourceRelative = Get-ResearchRelativePath $markdown.FullName
            if ($resolved.IndexOf((Join-Path $RepoRoot "artifacts"), [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
                $warnings.Add("${sourceRelative}: generated artifact target is not present: $rawTarget.")
            } else {
                $errors.Add("${sourceRelative}: missing local link target: $rawTarget.")
            }
        }
    }
}

$narratives = @(Get-ChildItem -LiteralPath $scanRoot -Recurse -Filter "narrative.md" -File -ErrorAction SilentlyContinue)
foreach ($narrative in $narratives) {
    $content = Get-Content -LiteralPath $narrative.FullName -Raw -Encoding UTF8
    $lines = @(Get-Content -LiteralPath $narrative.FullName -Encoding UTF8)
    $relative = Get-ResearchRelativePath $narrative.FullName
    $bodyLines = @($lines | Where-Object { $_.Trim() -ne "" -and $_ -notmatch '^\s*#' -and $_ -notmatch '^\s*```' })
    $bulletLines = @($bodyLines | Where-Object { $_ -match '^\s*[-*]\s+' })
    $localEvidenceLinks = [regex]::Matches($content, '\[[^\]]+\]\((?!https?://|mailto:)[^)]+\)').Count

    if ($bodyLines.Count -ge 12 -and $bulletLines.Count -ge 8 -and ($bulletLines.Count * 2) -gt $bodyLines.Count) {
        $warnings.Add("${relative}: narrative body is dominated by bullets; verify that it is a connected scientific manuscript rather than a QA outline.")
    }
    if ($bodyLines.Count -ge 12 -and $localEvidenceLinks -eq 0) {
        $warnings.Add("${relative}: narrative contains no local evidence links.")
    }
    $qaLabels = [regex]::Matches($content, '(?im)^\s*[-*]\s+\*\*[^*]+:\*\*').Count
    if ($qaLabels -ge 8) {
        $warnings.Add("${relative}: repeated labeled cards detected; keep gate QA structure in the evidence report and write the narrative as causal prose.")
    }
}

if (Test-Path -LiteralPath $statusPath -PathType Leaf) {
    foreach ($line in Get-Content -LiteralPath $statusPath -Encoding UTF8) {
        if ($line -match "archive/_active/([^/\s)]+)(?:/|\))") {
            $matchedProgramSlug = $Matches[1]
            if (([string]::IsNullOrWhiteSpace($ProgramSlug) -or $matchedProgramSlug -eq $ProgramSlug) -and $line -match "(?i)(archived|completed|finished|rejected)") {
                $warnings.Add("Status map uses a terminal label for active program '$matchedProgramSlug'.")
            }
        }
    }
}

foreach ($warning in $warnings) {
    Write-Warning $warning
}
foreach ($errorMessage in $errors) {
    Write-Host "ERROR: $errorMessage" -ForegroundColor Red
}

if ([string]::IsNullOrWhiteSpace($ProgramSlug)) {
    Write-Host "Checked the complete research tree."
} else {
    Write-Host "Focused check: active program '$ProgramSlug'."
}
Write-Host "Checked $($readmes.Count) research READMEs, $($narratives.Count) narratives, $($markdownFiles.Count) Markdown files, and $linkCount local links."
Write-Host "Warnings: $($warnings.Count); Errors: $($errors.Count)."

if ($errors.Count -gt 0) {
    exit 1
}
