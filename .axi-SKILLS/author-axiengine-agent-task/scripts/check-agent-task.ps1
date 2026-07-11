param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$resolvedPath = [System.IO.Path]::GetFullPath($Path)
if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
    Write-Error "Task file not found: $resolvedPath"
}

$content = Get-Content -LiteralPath $resolvedPath -Raw -Encoding UTF8
$lines = @(Get-Content -LiteralPath $resolvedPath -Encoding UTF8)
$warnings = [System.Collections.Generic.List[string]]::new()
$errors = [System.Collections.Generic.List[string]]::new()

function Add-Issue([string]$Message, [bool]$Required = $false) {
    if ($Strict -and $Required) {
        $errors.Add($Message)
    } else {
        $warnings.Add($Message)
    }
}

$fileName = [System.IO.Path]::GetFileName($resolvedPath)
if ($content -notmatch '(?m)^#\s+[A-Z][A-Z0-9]*[0-9]+(?:[a-z])?\b') {
    Add-Issue "${fileName}: title does not begin with a recognizable task ID." $true
}
if ($content -notmatch '(?im)^STATUS:\s*(DRAFT|READY|BLOCKED|ACCEPTED|DONE)\b') {
    Add-Issue "${fileName}: missing recognized STATUS metadata." $true
}
if ($content -notmatch '(?im)^TYPE:\s*(IMPLEMENTATION|RESEARCH|SPECIFICATION|REVIEW|DESIGN)\b') {
    Add-Issue "${fileName}: missing TYPE metadata; new tasks must declare one primary profile." $true
}
if ($content -notmatch '(?im)^SKILLS?:\s*.+$') {
    Add-Issue "${fileName}: missing exact applicable SKILLS metadata." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+(Goal|\u0426\u0435\u043b\u044c|Objective)\b') {
    Add-Issue "${fileName}: missing primary objective section." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+.*(?:Source|\u0418\u0441\u0442\u043e\u0447|Normative|\u041d\u043e\u0440\u043c\u0430\u0442\u0438\u0432)') {
    Add-Issue "${fileName}: no explicit source-of-truth section was found." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+.*(?:Scope|Work|Deliverables|\u0421\u0434\u0435\u043b\u0430\u0442\u044c|\u0420\u0430\u0431\u043e\u0442|\u041e\u0431\u044a[\u0435\u0451]\u043c)') {
    Add-Issue "${fileName}: no ordered scope or deliverables section was found." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+.*(?:Out of scope|\u041d\u0435\s+\u0434\u0435\u043b\u0430\u0442\u044c|\u0412\u043d\u0435\s+scope)') {
    Add-Issue "${fileName}: missing explicit out-of-scope section." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+.*(?:Acceptance|Done when|\u0413\u043e\u0442\u043e\u0432\u043e\s+\u043a\u043e\u0433\u0434\u0430|\u041f\u0440\u0438[\u0435\u0451]\u043c\u043a|Pass bar)') {
    Add-Issue "${fileName}: missing acceptance section." $true
}
if ($content -notmatch '(?im)^#{2,3}\s+.*(?:Handoff|\u0421\u0434\u0430\u0442\u044c|\u041e\u0442\u0447[\u0435\u0451]\u0442)') {
    Add-Issue "${fileName}: missing handoff requirements." $false
}

foreach ($match in [regex]::Matches($content, '(?im)(?:file:///[^\s)]+|\b[A-Za-z]:[\\/][^\s)`]+)')) {
    Add-Issue "${fileName}: machine-specific absolute path: $($match.Value)" $true
}
if ($content -match '(?i)\b(TODO|TBD|FIXME|fill this|decide later)\b') {
    Add-Issue "${fileName}: unresolved placeholder remains in task text." $true
}

$isResearch = $content -match '(?im)^TYPE:\s*RESEARCH\b|\b(preregister|preregistration|hypothesis|experiment|research)\b'
$isImplementation = $content -match '(?im)^TYPE:\s*IMPLEMENTATION\b|\bcargo\s+(?:test|check|clippy|fmt)\b|AxiEngine/crates/'
$isSpecification = $content -match '(?im)^TYPE:\s*SPECIFICATION\b|docs/engine/spec_L'

if ($isResearch) {
    if ($content -notmatch 'conduct-axiengine-research') {
        Add-Issue "${fileName}: research task does not name conduct-axiengine-research explicitly." $true
    }
    foreach ($term in @('prereg', 'control', 'verdict')) {
        if ($content -notmatch "(?i)$term") {
            Add-Issue "${fileName}: research contract may be missing '$term' semantics." $true
        }
    }
    if ($content -notmatch '(?i)\blimits?\b|claim boundary|cannot establish|unknowns?|out of scope') {
        Add-Issue "${fileName}: research contract may be missing explicit claim limits." $true
    }
    $gateCount = [regex]::Matches($content, '(?im)^\|\s*\*\*?[A-Z][0-9]+\*\*?\s*\|').Count
    if ($gateCount -ge 2 -and $content -notmatch '(?i)narrative\.md|short single-gate') {
        Add-Issue "${fileName}: multi-gate research task omits living narrative.md or an explicit short single-gate exception." $true
    }
    if ($content -match '(?i)\b(parity|transfer|replay|identical inputs)\b') {
        if ($content -notmatch '(?i)input[- ]equivalence|input mapping|control[- ]flow|preconditions?|early exits?') {
            Add-Issue "${fileName}: parity task lacks whole-path input/control-flow equivalence mapping." $true
        }
        if ($content -notmatch '(?i)sanity assert|assertion|independent output validation|branch selection') {
            Add-Issue "${fileName}: parity runner lacks required sanity assertions or independent output validation." $true
        }
    }
}

if ($isImplementation) {
    if ($content -notmatch 'implement-axiengine-rust-change') {
        Add-Issue "${fileName}: implementation task does not name implement-axiengine-rust-change explicitly." $true
    }
    if ($content -notmatch '(?i)cargo\s+(test|check|build|clippy|fmt)') {
        Add-Issue "${fileName}: implementation task has no exact cargo verification command." $true
    }
}

if ($isSpecification -and $content -notmatch 'write-axiengine-crate-spec') {
    Add-Issue "${fileName}: specification task does not name write-axiengine-crate-spec explicitly." $true
}

foreach ($warning in $warnings) {
    Write-Warning $warning
}
foreach ($errorMessage in $errors) {
    Write-Host "ERROR: $errorMessage" -ForegroundColor Red
}

Write-Host "Checked task '$fileName': $($lines.Count) lines, $($warnings.Count) warnings, $($errors.Count) errors."
if ($errors.Count -gt 0) {
    exit 1
}
