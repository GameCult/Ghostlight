[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string[]]$RunRoot,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'

function Get-Percentile([double[]]$Values, [double]$Percentile) {
    if ($Values.Count -eq 0) { return 0 }
    $sorted = @($Values | Sort-Object)
    $index = [math]::Ceiling($Percentile * $sorted.Count) - 1
    $sorted[[math]::Max(0, [math]::Min($index, $sorted.Count - 1))]
}

function Get-ScenarioKind([string]$ScenarioId) {
    switch -Regex ($ScenarioId) {
        'compiler' { 'compiler'; break }
        'live[-_]turn' { 'live_turn'; break }
        'action' { 'action'; break }
        'strategic' { 'strategic'; break }
        'scale' { 'scale'; break }
        'gestalt[-_]dynamics' { 'gestalt_dynamics'; break }
        default { 'unknown' }
    }
}

function Get-PlanActionCount($Plan) {
    if (-not $Plan) { return 0 }
    $count = 0
    foreach ($property in @('institution_actions', 'gestalt_actions', 'gestalt_activities', 'actor_moves', 'member_migrations')) {
        if ($Plan.PSObject.Properties.Name -contains $property) {
            $count += @($Plan.$property).Count
        }
    }
    $count
}

$scenarios = [Collections.Generic.List[object]]::new()
$stages = [Collections.Generic.List[object]]::new()
$seenResults = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)

foreach ($rootValue in $RunRoot) {
    $root = [IO.Path]::GetFullPath($rootValue)
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        throw "Live-fire run root is absent: $root"
    }
    $summaryById = @{}
    $summaryPath = Join-Path $root 'summary.jsonl'
    if (Test-Path -LiteralPath $summaryPath -PathType Leaf) {
        foreach ($line in Get-Content -LiteralPath $summaryPath) {
            if (-not [string]::IsNullOrWhiteSpace($line)) {
                $summary = $line | ConvertFrom-Json -Depth 100
                $summaryById[$summary.scenario_id] = $summary
            }
        }
    }

    $resultFiles = @()
    $directResult = Join-Path $root 'result.json'
    if (Test-Path -LiteralPath $directResult -PathType Leaf) {
        $resultFiles += Get-Item -LiteralPath $directResult
    }
    $resultFiles += @(Get-ChildItem -LiteralPath (Join-Path $root 'results') -Filter result.json -File -Recurse -ErrorAction SilentlyContinue)
    foreach ($file in $resultFiles) {
        $resultKey = $file.FullName
        if (-not $seenResults.Add($resultKey)) { continue }
        $result = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json -Depth 100
        $scenarioId = [string]$result.scenario_id
        $summary = $summaryById[$scenarioId]
        $kind = if ($summary) { [string]$summary.kind } else { Get-ScenarioKind $scenarioId }
        $elapsed = if ($result.PSObject.Properties.Name -contains 'elapsed_seconds') {
            [double]$result.elapsed_seconds
        } elseif ($result.PSObject.Properties.Name -contains 'total_seconds') {
            [double]$result.total_seconds
        } else { 0 }
        $actionCount = Get-PlanActionCount $result.plan
        if ($result.PSObject.Properties.Name -contains 'sustained_waves') {
            foreach ($wave in @($result.sustained_waves)) {
                $actionCount += Get-PlanActionCount $wave.plan
            }
        }
        $scenarios.Add([pscustomobject]@{
            run_root = $root
            scenario_id = $scenarioId
            kind = $kind
            succeeded = $true
            elapsed_seconds = $elapsed
            verdict = if ($summary) { $summary.verdict } else { $null }
            result_path = $file.FullName
            stderr = $null
            action_count = $actionCount
        })

        $receipts = @()
        foreach ($property in @('model_receipts', 'model_stage_receipts')) {
            if ($result.PSObject.Properties.Name -contains $property) {
                $receipts = @($result.$property)
                break
            }
        }
        if ($result.PSObject.Properties.Name -contains 'sustained_waves') {
            foreach ($wave in @($result.sustained_waves)) {
                if ($wave.PSObject.Properties.Name -contains 'model_stage_receipts') {
                    $receipts += @($wave.model_stage_receipts)
                }
            }
        }
        foreach ($receipt in $receipts) {
            $prompt = 0L
            $hit = 0L
            $miss = 0L
            $completion = 0L
            $providerLatency = 0L
            $attemptCount = 0
            foreach ($attempt in @($receipt.provider_attempts)) {
                $attemptCount++
                $providerLatency += [long]$attempt.latency_ms
                if ($attempt.token_usage) {
                    $prompt += [long]$attempt.token_usage.prompt_tokens
                    $hit += [long]$attempt.token_usage.prompt_cache_hit_tokens
                    $miss += [long]$attempt.token_usage.prompt_cache_miss_tokens
                    $completion += [long]$attempt.token_usage.completion_tokens
                }
            }
            $stages.Add([pscustomobject]@{
                run_root = $root
                scenario_id = $scenarioId
                kind = $kind
                stage = [string]$receipt.stage
                model = [string]$receipt.model
                validation = [string]$receipt.validation_result
                accepted = ([string]$receipt.validation_result).StartsWith('valid')
                latency_ms = [long]$receipt.latency_ms
                provider_latency_ms = $providerLatency
                input_chars = [long]$receipt.input_chars
                output_chars = [long]$receipt.output_chars
                attempts = $attemptCount
                prompt_tokens = $prompt
                cache_hit_tokens = $hit
                cache_miss_tokens = $miss
                completion_tokens = $completion
            })
        }
    }

    foreach ($summary in $summaryById.Values) {
        if ($summary.succeeded) { continue }
        $stderr = if ($summary.stderr_log -and (Test-Path -LiteralPath $summary.stderr_log)) {
            (Get-Content -LiteralPath $summary.stderr_log -Raw).Trim()
        } else { $null }
        $scenarios.Add([pscustomobject]@{
            run_root = $root
            scenario_id = [string]$summary.scenario_id
            kind = [string]$summary.kind
            succeeded = $false
            elapsed_seconds = 0
            verdict = $summary.verdict
            result_path = $null
            stderr = $stderr
            action_count = 0
        })
    }
}

$scenarioGroups = @($scenarios | Group-Object kind | Sort-Object Name | ForEach-Object {
    $elapsed = @($_.Group | Where-Object succeeded | ForEach-Object { [double]$_.elapsed_seconds })
    [pscustomobject]@{
        kind = $_.Name
        scenarios = $_.Count
        succeeded = @($_.Group | Where-Object succeeded).Count
        failed = @($_.Group | Where-Object { -not $_.succeeded }).Count
        committed_actions = [long](($_.Group | Measure-Object action_count -Sum).Sum)
        p50_elapsed_seconds = [math]::Round((Get-Percentile $elapsed 0.50), 3)
        p95_elapsed_seconds = [math]::Round((Get-Percentile $elapsed 0.95), 3)
        max_elapsed_seconds = [math]::Round((Get-Percentile $elapsed 1.00), 3)
    }
})

$stageGroups = @($stages | Group-Object stage, model | Sort-Object Name | ForEach-Object {
    $group = @($_.Group)
    $prompt = [long](($group | Measure-Object prompt_tokens -Sum).Sum)
    $hit = [long](($group | Measure-Object cache_hit_tokens -Sum).Sum)
    $completion = [long](($group | Measure-Object completion_tokens -Sum).Sum)
    $outputChars = [long](($group | Measure-Object output_chars -Sum).Sum)
    [pscustomobject]@{
        stage = $group[0].stage
        model = $group[0].model
        receipts = $group.Count
        accepted = @($group | Where-Object accepted).Count
        semantic_invalid = @($group | Where-Object { $_.validation -eq 'semantic_invalid' }).Count
        provider_attempts = [long](($group | Measure-Object attempts -Sum).Sum)
        p50_latency_ms = [long](Get-Percentile @($group | ForEach-Object { [double]$_.latency_ms }) 0.50)
        p95_latency_ms = [long](Get-Percentile @($group | ForEach-Object { [double]$_.latency_ms }) 0.95)
        input_chars = [long](($group | Measure-Object input_chars -Sum).Sum)
        output_chars = $outputChars
        prompt_tokens = $prompt
        cache_hit_tokens = $hit
        cache_miss_tokens = [long](($group | Measure-Object cache_miss_tokens -Sum).Sum)
        completion_tokens = $completion
        cache_hit_ratio = if ($prompt -gt 0) { [math]::Round($hit / $prompt, 4) } else { 0 }
        output_chars_per_completion_token = if ($completion -gt 0) { [math]::Round($outputChars / $completion, 3) } else { 0 }
    }
})

$totalPrompt = [long](($stages | Measure-Object prompt_tokens -Sum).Sum)
$totalHit = [long](($stages | Measure-Object cache_hit_tokens -Sum).Sum)
$totalCompletion = [long](($stages | Measure-Object completion_tokens -Sum).Sum)
$totalActions = [long](($scenarios | Measure-Object action_count -Sum).Sum)
$invalidStages = @($stages | Where-Object { -not $_.accepted })
$report = [ordered]@{
    schema = 'ghostlight.live_fire_profile.v1'
    generated_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
    run_roots = @($RunRoot | ForEach-Object { [IO.Path]::GetFullPath($_) })
    totals = [ordered]@{
        scenarios = $scenarios.Count
        succeeded = @($scenarios | Where-Object succeeded).Count
        failed = @($scenarios | Where-Object { -not $_.succeeded }).Count
        stage_receipts = $stages.Count
        provider_attempts = [long](($stages | Measure-Object attempts -Sum).Sum)
        prompt_tokens = $totalPrompt
        cache_hit_tokens = $totalHit
        cache_miss_tokens = [long](($stages | Measure-Object cache_miss_tokens -Sum).Sum)
        completion_tokens = $totalCompletion
        committed_actions = $totalActions
        prompt_tokens_per_committed_action = if ($totalActions -gt 0) { [math]::Round($totalPrompt / $totalActions, 1) } else { 0 }
        completion_tokens_per_committed_action = if ($totalActions -gt 0) { [math]::Round($totalCompletion / $totalActions, 1) } else { 0 }
        cache_hit_ratio = if ($totalPrompt -gt 0) { [math]::Round($totalHit / $totalPrompt, 4) } else { 0 }
        semantic_invalid_receipts = @($stages | Where-Object { $_.validation -eq 'semantic_invalid' }).Count
        rejected_prompt_tokens = [long](($invalidStages | Measure-Object prompt_tokens -Sum).Sum)
        rejected_completion_tokens = [long](($invalidStages | Measure-Object completion_tokens -Sum).Sum)
    }
    by_kind = $scenarioGroups
    by_stage = $stageGroups
    failures = @($scenarios | Where-Object { -not $_.succeeded } | Select-Object run_root, scenario_id, kind, stderr)
}

$json = $report | ConvertTo-Json -Depth 12
if ($OutputPath) {
    $parent = Split-Path -Parent ([IO.Path]::GetFullPath($OutputPath))
    if ($parent) { [IO.Directory]::CreateDirectory($parent) | Out-Null }
    [IO.File]::WriteAllText([IO.Path]::GetFullPath($OutputPath), $json)
}
$json
