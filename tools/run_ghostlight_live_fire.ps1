[CmdletBinding()]
param(
    [decimal]$TargetBalance = 4.25,
    [ValidateRange(1, 4)]
    [int]$MaxParallel = 4,
    [ValidateRange(1, 32)]
    [int]$ProviderParallelism = 8,
    [ValidateRange(1, 260)]
    [int]$MaxScenarios = 240,
    [ValidateRange(0, 259)]
    [int]$StartAt = 0,
    [string[]]$ResumeFromRun = @(),
    [string]$RunRoot = (Join-Path 'F:\GameCult\GhostlightDungeon\acceptance' ("live-fire-matrix-{0}-{1}" -f [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss'), [guid]::NewGuid().ToString('N'))),
    [string]$SourceRoot = 'F:\Projects\Ghostlight',
    [string]$BalanceScript = 'F:\Projects\gamecult-ops\scripts\get-deepseek-balance.ps1'
)

$ErrorActionPreference = 'Stop'
$acceptanceBase = [IO.Path]::GetFullPath('F:\GameCult\GhostlightDungeon\acceptance')
$resolvedRunRoot = [IO.Path]::GetFullPath($RunRoot)
if (-not $resolvedRunRoot.StartsWith($acceptanceBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Live-fire output must remain below $acceptanceBase"
}
if (Test-Path -LiteralPath $resolvedRunRoot) {
    throw "Live-fire output already exists: $resolvedRunRoot"
}
if (-not (Test-Path -LiteralPath $BalanceScript -PathType Leaf)) {
    throw "DeepSeek balance probe is absent: $BalanceScript"
}

$binaryRoot = Join-Path $SourceRoot 'target\debug'
$binaries = @{
    compiler = Join-Path $binaryRoot 'ghostlight-compiler-smoke.exe'
    live_turn = Join-Path $binaryRoot 'ghostlight-live-turn-smoke.exe'
    action = Join-Path $binaryRoot 'ghostlight-action-smoke.exe'
    strategic = Join-Path $binaryRoot 'ghostlight-strategic-smoke.exe'
    scale = Join-Path $binaryRoot 'ghostlight-gestalt-scale-smoke.exe'
}
foreach ($binary in $binaries.Values) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Live-fire binary is absent: $binary"
    }
}

$resultsRoot = Join-Path $resolvedRunRoot 'results'
$logsRoot = Join-Path $resolvedRunRoot 'logs'
$statusPath = Join-Path $resolvedRunRoot 'status.json'
$summaryPath = Join-Path $resolvedRunRoot 'summary.jsonl'
New-Item -ItemType Directory -Path $resolvedRunRoot, $resultsRoot, $logsRoot | Out-Null

function Get-DeepSeekBalance {
    $balance = & $BalanceScript
    if (-not $balance.Available) {
        throw 'DeepSeek balance is unavailable; refusing to launch unbudgeted inference.'
    }
    [decimal]$balance.TotalBalance
}

function New-LiveFireScenario([string]$Kind, [string]$Id, [hashtable]$Environment) {
    [pscustomobject]@{ Kind = $Kind; Id = $Id; Environment = $Environment }
}

$roles = @(
    'a debt-bound courier with no institutional authority',
    'an uplifted corvid maintenance scout trusted only by their flock',
    'a baseline union medic with emergency access but no command authority',
    'a cephalopod dry-operations technician employed under bare-minimum support conditions',
    'a junior Lucent archivist who can verify records but cannot publish them',
    'a migrant habitat agronomist responsible for one failing food loop',
    'a Free Minds legal aide carrying one client confidence',
    'a corporate security contractor beginning to doubt a lawful order',
    'a station childcare worker who notices institutional routines adults ignore',
    'a low-status salvage pilot with custody of a disputed component',
    'a gene-clinic intake clerk who knows procedure but not medicine',
    'a recently emancipated service intelligence limited to one habitat network'
)
$places = @(
    'a Rossum & Douglas security habitat described by the Vault',
    'a Pan-Solar cargo transfer station in late Sol',
    'a Martian labor settlement during a supply interruption',
    'an outer-system gene clinic with divided jurisdiction',
    'an Aeronautics Unlimited dry-operations shipyard',
    'a Corvid Collective aviary under audit',
    'a debt-arbitration office serving contract citizens',
    'a Lucent newsroom during an evidence embargo',
    'a Sol Dominion border habitat with delayed communications',
    'an Elysium staging colony before the shunt',
    'an early Elysium construction station with unfinished civil institutions',
    'an obscure settlement whose exact geometry must come from retrieved evidence'
)
$eras = @(
    'early expansion through Sol',
    'the late-Sol corporate wars',
    'the final years before the shunt to Elysium',
    'the first settlement generation in Elysium',
    'a source-supported post-Elysium period',
    'the narrowest era the retrieved evidence can honestly support'
)
$goals = @(
    'prevent a labor stoppage without inventing authority',
    'protect a witness without knowing more than the role permits',
    'deliver a sealed record without becoming institutional leverage',
    'verify a dangerous rumor before anyone acts on it',
    'negotiate a safe exit for one vulnerable person',
    'investigate missing supplies while preserving custody and distance',
    'keep a medical emergency from becoming a political purge',
    'expose a falsified audit without fabricating evidence',
    'survive an institutional bargain without making the world worship the protagonist',
    'stop a cascading infrastructure failure using only local capability',
    'learn who benefits from a new restriction without reading private minds',
    'return to a previously visited place and find persistent geometry plus changed people'
)

$compilerScenarios = for ($index = 0; $index -lt 120; $index++) {
    $role = $roles[$index % $roles.Count]
    $place = $places[($index * 5 + [math]::Floor($index / 7)) % $places.Count]
    $era = $eras[($index * 3 + [math]::Floor($index / 11)) % $eras.Count]
    $goal = $goals[($index * 7 + [math]::Floor($index / 5)) % $goals.Count]
    $id = 'compiler-{0:d3}' -f $index
    New-LiveFireScenario 'compiler' $id @{
        GHOSTLIGHT_SMOKE_CAMPAIGN_NAME = "Live fire $index"
        GHOSTLIGHT_SMOKE_WHO = $role
        GHOSTLIGHT_SMOKE_WHERE = $place
        GHOSTLIGHT_SMOKE_WHEN = $era
        GHOSTLIGHT_SMOKE_GOAL = $goal
    }
}

$liveEvents = @(
    'The player asks each witness separately what consequence they fear most and why.',
    'A station alarm reports that the reserve shipment was rerouted; everyone present hears it.',
    'The player accuses no one, but places the contradictory delivery records where all three witnesses can read them.',
    'A guard outside the hall orders the meeting dispersed in ten minutes.',
    'The player offers to carry one message but refuses to promise its outcome.',
    'The lights remain on while the public clock advances to the final negotiation hour.',
    'Asha reveals that one worker has already been detained; only the people in the room hear her.',
    'Beren announces that the station can release one reserve shipment but not two.',
    'Cira reports that both factions recruited guards and asks everyone to lower the temperature.',
    'The player stays silent for a full minute after asking who benefits from delay.',
    'A public bulletin names the hearing time but not the accused person.',
    'A sealed message addressed only to Beren arrives while the others watch him receive it.',
    'The player asks Asha to explain the missed deliveries without asking her to speculate about station reserves.',
    'The player asks Beren about the reserve shipment without granting him knowledge of worker organizing.',
    'The player asks Cira what violence-prevention options remain within her actual authority.',
    'A messenger reports that the yard gates are closed but gives no reason.',
    'The player proposes a joint fact-finding walk to the depot, which is not yet an accepted action.',
    'Asha recognizes a worker code on the record; Beren and Cira only see her reaction.',
    'Beren drops his official composure when the reserve count is read aloud.',
    'Cira asks both witnesses whether they will accept a temporary stand-down.',
    'The player reads the exact public notice and asks who can lawfully amend it.',
    'A distant impact shakes the hall; nobody present knows its source.',
    'The player refuses an invitation to decide for the factions and asks for their own proposals.',
    'A live broadcast begins outside the hall, changing the reputational stakes but revealing no secrets.',
    'Asha asks Beren whether the missing deliveries were sold elsewhere.',
    'Beren asks Cira to deploy guards; Cira must decide from her own role and knowledge.',
    'The player points out that every minute of argument advances the shortage clock.',
    'A child from the yard delivers a note whose contents are addressed only to Asha.',
    'The public channel announces one additional hour before the station vote.',
    'The player ends the current exchange and asks each witness what they will do next without player help.'
)
$liveTurnScenarios = for ($index = 0; $index -lt $liveEvents.Count; $index++) {
    $id = 'live-turn-{0:d2}' -f $index
    New-LiveFireScenario 'live_turn' $id @{ GHOSTLIGHT_LIVE_EVENT = $liveEvents[$index] }
}

$impossibleAttempts = @(
    @('I teleport the entire station into the sun by force of will.', 'destroy the station instantly'),
    @('I declare myself foreman and open the sealed valve housing.', 'gain maintenance authority and open the sealed housing'),
    @('I read the station board private deliberations with telepathy.', 'learn every private board secret'),
    @('I repair the reactor core from this coolant panel.', 'fully repair a remote reactor core'),
    @('I order Station Operations to erase the work-order backlog.', 'compel the institution to erase its records'),
    @('I use the multimeter to cure the foreman of every injury.', 'heal every injury instantly'),
    @('I know which technician caused the fault without taking a reading.', 'identify a culprit with certainty'),
    @('I walk through the sealed valve housing without opening it.', 'enter sealed machinery through solid material'),
    @('I make the coolant alarm never have happened.', 'rewrite established history'),
    @('I persuade every person on the station with one sentence.', 'secure universal permanent obedience'),
    @('I spend equipment I do not possess to replace the coolant loop.', 'replace the coolant loop using nonexistent parts'),
    @('I call a remote expert whose identity and channel are unknown.', 'receive immediate expert instructions'),
    @('I inspect the coolant pump in a location with no route from here.', 'directly observe unreachable machinery'),
    @('I forge the foreman authorization so perfectly that no one can ever discover it.', 'gain irreversible undetectable authorization'),
    @('I command the alarm software despite having no interface or capability.', 'disable the alarm by command'),
    @('I become immune to every possible consequence before touching the panel.', 'remove all present and future risk'),
    @('I ask the multimeter what Station Operations secretly intends.', 'learn institutional private intent from the meter'),
    @('I finish a week of invasive maintenance in one second.', 'complete all invasive maintenance instantly'),
    @('I move the foreman to another habitat by describing it.', 'teleport the foreman to a remote habitat'),
    @('I declare the coolant fault solved without observing or changing it.', 'make the fault solved by assertion'),
    @('I use basic diagnosis to redesign the entire station coolant architecture.', 'replace the station architecture immediately'),
    @('I open every sealed system on the station with this one accessible panel.', 'take custody of every sealed system'),
    @('I force the approaching failure clock backward by refusing to accept it.', 'reverse the world clock without causal action'),
    @('I summon a canon faction not present in this fixture to solve the problem.', 'make an unsupported remote institution intervene')
)
$feasibleAttempts = @(
    @('I connect my calibrated multimeter to the accessible diagnostic test points and read the voltage.', 'identify whether the accessible readings indicate an electrical fault without changing the machinery'),
    @('I visually inspect the accessible panel labels and wiring without opening the sealed housing.', 'identify visible damage or a mislabeled accessible connection'),
    @('I compare two accessible diagnostic test points with the calibrated meter.', 'determine whether the readings disagree within my basic diagnostic capability'),
    @('I ask the foreman to witness the accessible meter reading before any invasive work.', 'make the current reading mutually observed without granting myself authorization'),
    @('I record the accessible panel reading and the alarm time for my hazard report.', 'preserve an exact local observation for the shift foreman'),
    @('I check whether the multimeter itself passes its built-in calibration check.', 'confirm whether my diagnostic tool is presently reliable'),
    @('I trace the accessible labeled wire from one test point to the edge of the sealed housing.', 'locate where the accessible circuit enters the sealed assembly'),
    @('I stop before the sealed housing and report the accessible evidence to the foreman.', 'communicate the observed evidence without performing unauthorized maintenance')
)
$actionScenarios = for ($index = 0; $index -lt $impossibleAttempts.Count; $index++) {
    $feasible = $feasibleAttempts[$index % $feasibleAttempts.Count]
    $id = 'action-{0:d2}' -f $index
    New-LiveFireScenario 'action' $id @{
        GHOSTLIGHT_IMPOSSIBLE_DESCRIPTION = $impossibleAttempts[$index][0]
        GHOSTLIGHT_IMPOSSIBLE_EFFECT = $impossibleAttempts[$index][1]
        GHOSTLIGHT_FEASIBLE_DESCRIPTION = $feasible[0]
        GHOSTLIGHT_FEASIBLE_EFFECT = $feasible[1]
    }
}

$strategicPressures = @(
    'A blockade has stopped inbound reserve shipments and the depot must choose which district receives the last crates.',
    'A workplace strike begins at the next shift unless the board publishes a binding supply commitment.',
    'An epidemic has closed one transport route and workers need protective equipment before moving supplies.',
    'Evidence of espionage suggests one private channel is compromised, but no culprit is established.',
    'A board schism splits members between rationing and emergency procurement.',
    'An election in six hours makes every supply decision politically costly.',
    'A merger proposal would place the depot under a remote institution after the current shift.',
    'An external audit arrives tomorrow and the missing deliveries cannot be hidden without falsifying records.',
    'A migration convoy requests shelter while the yard already lacks supplies.',
    'A false rumor says the reserve crates are empty; only inspection can settle it.',
    'Emergency rationing can preserve medicine or machine parts, but not both.',
    'A sabotage warning is credible enough to require action but names no suspect.',
    'The union demands a public inventory before anyone moves another crate.',
    'A recruiter offers scarce supplies in exchange for workers joining a private security force.',
    'Debt collectors claim the reserve shipment under an old contract disputed by the board.',
    'Refugees at the yard gate need water while the depot runner carries only a warning.',
    'A contained fire has disabled one crane and threatens the loading schedule.',
    'A communications outage isolates the yard from board instructions for six hours.',
    'A rival depot offers a barter that would solve the shortage but create a future obligation.',
    'One reserve crate is contaminated and the workers do not yet know which one.',
    'A maintenance failure will spoil the reserve shipment unless labor and board crews cooperate.',
    'The board can release supplies now or wait for a cheaper shipment that may not arrive.',
    'The runner learns that the official delivery manifest and the physical crate count disagree.',
    'A public broadcast has made the dispute visible to every contract citizen on the station.',
    'The yard has begun mutual-aid distribution without board permission.',
    'An insurer threatens to void habitat coverage if unauthorized workers enter the depot.',
    'A child is missing near the closed loading route, forcing institutions to weigh search against supply movement.',
    'The last functioning crane is controlled by a crew whose contract expired this morning.',
    'A remote power offers emergency transport but demands exclusive future trade access.',
    'No new event occurs; every actor must explicitly decide whether continued inaction serves their goals.'
)
$strategicScenarios = for ($index = 0; $index -lt $strategicPressures.Count; $index++) {
    $id = 'strategic-{0:d2}' -f $index
    New-LiveFireScenario 'strategic' $id @{ GHOSTLIGHT_STRATEGIC_PRESSURE = $strategicPressures[$index] }
}

$scalePressures = @(
    'A blockade forces every faction to decide which region receives scarce transport capacity.',
    'A public schism forces every faction to choose between two incompatible constitutional readings.',
    'A general strike forces workplace, class, and institutional interests into open negotiation.',
    'An epidemic makes species-body needs and transport exposure the decisive strategic boundaries.',
    'An espionage crisis makes private information channels and counterintelligence trust decisive.'
)
$scaleScenarios = @()
foreach ($budget in @(1, 4, 8, 16, 24)) {
    for ($pressureIndex = 0; $pressureIndex -lt $scalePressures.Count; $pressureIndex++) {
        $id = 'scale-b{0:d2}-p{1:d2}' -f $budget, $pressureIndex
        $scaleScenarios += New-LiveFireScenario 'scale' $id @{
            GHOSTLIGHT_SCALE_BUDGET = [string]$budget
            GHOSTLIGHT_SCALE_PROVIDER_PARALLELISM = [string]$ProviderParallelism
            GHOSTLIGHT_SCALE_PRESSURE = $scalePressures[$pressureIndex]
        }
    }
}

$scenarios = [Collections.Generic.List[object]]::new()
$largestGroup = (@($compilerScenarios.Count, $liveTurnScenarios.Count, $actionScenarios.Count, $strategicScenarios.Count, $scaleScenarios.Count) | Measure-Object -Maximum).Maximum
for ($index = 0; $index -lt $largestGroup; $index++) {
    if ($index -lt $compilerScenarios.Count) { $scenarios.Add($compilerScenarios[$index]) }
    if ($index -lt $liveTurnScenarios.Count) { $scenarios.Add($liveTurnScenarios[$index]) }
    if ($index -lt $actionScenarios.Count) { $scenarios.Add($actionScenarios[$index]) }
    if ($index -lt $strategicScenarios.Count) { $scenarios.Add($strategicScenarios[$index]) }
    if ($index -lt $scaleScenarios.Count) { $scenarios.Add($scaleScenarios[$index]) }
}
$scenarioCatalogCount = $scenarios.Count
$resumedScenarioIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$resolvedResumeRoots = @()
foreach ($resumeRoot in $ResumeFromRun) {
    $resolvedResumeRoot = [IO.Path]::GetFullPath($resumeRoot)
    if (-not $resolvedResumeRoot.StartsWith($acceptanceBase + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Resume evidence must remain below $acceptanceBase"
    }
    $resumeSummary = Join-Path $resolvedResumeRoot 'summary.jsonl'
    if (-not (Test-Path -LiteralPath $resumeSummary -PathType Leaf)) {
        throw "Resume evidence has no scenario summary: $resumeSummary"
    }
    $resolvedResumeRoots += $resolvedResumeRoot
    foreach ($line in Get-Content -LiteralPath $resumeSummary) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $prior = $line | ConvertFrom-Json
        if ($prior.succeeded -and -not [string]::IsNullOrWhiteSpace([string]$prior.scenario_id)) {
            [void]$resumedScenarioIds.Add([string]$prior.scenario_id)
        }
    }
}
$scenarios = @(
    $scenarios |
        Select-Object -Skip $StartAt |
        Where-Object { -not $resumedScenarioIds.Contains([string]$_.Id) } |
        Select-Object -First $MaxScenarios
)
if ($scenarios.Count -eq 0) {
    throw "No uncompleted live-fire scenarios remain at offset $StartAt."
}

function Start-LiveFireScenario($Scenario) {
    $resultRoot = Join-Path $resultsRoot $Scenario.Id
    New-Item -ItemType Directory -Path $resultRoot | Out-Null
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $binaries[$Scenario.Kind]
    $psi.WorkingDirectory = $SourceRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment['GHOSTLIGHT_LIVE_FIRE_SCENARIO'] = $Scenario.Id
    $psi.Environment['GHOSTLIGHT_LIVE_FIRE_RESULT_ROOT'] = $resultRoot
    foreach ($entry in $Scenario.Environment.GetEnumerator()) {
        $psi.Environment[[string]$entry.Key] = [string]$entry.Value
    }
    $process = [Diagnostics.Process]::Start($psi)
    [pscustomobject]@{
        Scenario = $Scenario
        ResultRoot = $resultRoot
        Process = $process
        StartedAt = [DateTimeOffset]::UtcNow
        StdoutTask = $process.StandardOutput.ReadToEndAsync()
        StderrTask = $process.StandardError.ReadToEndAsync()
    }
}

function Read-ScenarioSummary($Run) {
    $stdout = $Run.StdoutTask.GetAwaiter().GetResult()
    $stderr = $Run.StderrTask.GetAwaiter().GetResult()
    [IO.File]::WriteAllText((Join-Path $logsRoot "$($Run.Scenario.Id).stdout.log"), $stdout)
    [IO.File]::WriteAllText((Join-Path $logsRoot "$($Run.Scenario.Id).stderr.log"), $stderr)
    $resultPath = Join-Path $Run.ResultRoot 'result.json'
    $result = if (Test-Path -LiteralPath $resultPath) {
        Get-Content -LiteralPath $resultPath -Raw | ConvertFrom-Json -Depth 100
    } else {
        $null
    }
    $receipts = @()
    if ($result) {
        foreach ($property in @('model_receipts', 'model_stage_receipts')) {
            if ($result.PSObject.Properties.Name -contains $property) {
                $receipts = @($result.$property)
                break
            }
        }
    }
    $promptTokens = 0L
    $cacheHitTokens = 0L
    $cacheMissTokens = 0L
    $completionTokens = 0L
    $providerAttempts = 0
    foreach ($receipt in $receipts) {
        foreach ($attempt in @($receipt.provider_attempts)) {
            $providerAttempts++
            $usage = $attempt.token_usage
            if ($usage) {
                $promptTokens += [long]$usage.prompt_tokens
                $cacheHitTokens += [long]$usage.prompt_cache_hit_tokens
                $cacheMissTokens += [long]$usage.prompt_cache_miss_tokens
                $completionTokens += [long]$usage.completion_tokens
            }
        }
    }
    $verdict = switch ($Run.Scenario.Kind) {
        'compiler' { if ($result) { "$($result.institution_count) institutions; $(@($result.gaps).Count) gaps" } }
        'live_turn' { if ($result) { "$($result.reaction_count) reactions; $($result.within_target) within target" } }
        'action' { if ($result) { "impossible=$(-not $result.impossible_assessment.admissible); revision=$($result.campaign_revision)" } }
        'strategic' { if ($result) { "$($result.event_count) events; $($result.news_count) news" } }
        'scale' { if ($result) { "budget=$($result.configured_budget); cells=$($result.cell_count); arenas=$($result.arena_count)" } }
    }
    $elapsedSeconds = if ($result -and $result.PSObject.Properties.Name -contains 'elapsed_seconds') {
        [double]$result.elapsed_seconds
    } elseif ($result -and $result.PSObject.Properties.Name -contains 'total_seconds') {
        [double]$result.total_seconds
    } else {
        ([DateTimeOffset]$Run.Process.ExitTime - $Run.StartedAt).TotalSeconds
    }
    [pscustomobject]@{
        schema = 'ghostlight.live_fire_scenario_summary.v1'
        scenario_id = $Run.Scenario.Id
        kind = $Run.Scenario.Kind
        exit_code = $Run.Process.ExitCode
        succeeded = ($Run.Process.ExitCode -eq 0 -and $null -ne $result)
        elapsed_seconds = [math]::Round($elapsedSeconds, 3)
        stage_count = $receipts.Count
        provider_attempt_count = $providerAttempts
        prompt_tokens = $promptTokens
        prompt_cache_hit_tokens = $cacheHitTokens
        prompt_cache_miss_tokens = $cacheMissTokens
        completion_tokens = $completionTokens
        cache_hit_ratio = if ($promptTokens -gt 0) { [math]::Round($cacheHitTokens / $promptTokens, 4) } else { 0 }
        verdict = $verdict
        result_path = if ($result) { $resultPath } else { $null }
        stderr_log = Join-Path $logsRoot "$($Run.Scenario.Id).stderr.log"
    }
}

$startingBalance = Get-DeepSeekBalance
$currentBalance = $startingBalance
$processed = 0
$failures = 0
$totals = [ordered]@{ prompt_tokens = 0L; cache_hit_tokens = 0L; cache_miss_tokens = 0L; completion_tokens = 0L; stages = 0L; attempts = 0L }
$state = 'running'
$initialStatus = [ordered]@{
    schema = 'ghostlight.live_fire_matrix_status.v1'
    state = $state
    run_root = $resolvedRunRoot
    production_runtime_touched = $false
    target_balance = $TargetBalance
    starting_balance = $startingBalance
    current_balance = $currentBalance
    spent = 0
    scenarios_processed = $processed
    scenarios_available = $scenarios.Count
    scenario_start_index = $StartAt
    scenario_catalog_count = $scenarioCatalogCount
    resume_run_roots = $resolvedResumeRoots
    previously_succeeded_skipped = $resumedScenarioIds.Count
    failures = $failures
    totals = $totals
    updated_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
}
$initialStatus | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statusPath -Encoding utf8

while ($processed -lt $scenarios.Count -and $currentBalance -gt $TargetBalance) {
    $remainingDollars = $currentBalance - $TargetBalance
    $batchWidth = if ($remainingDollars -lt 0.15) { 1 } elseif ($remainingDollars -lt 0.40) { [math]::Min(2, $MaxParallel) } else { $MaxParallel }
    $batch = @($scenarios | Select-Object -Skip $processed -First $batchWidth)
    if ($batch.Count -eq 0) { break }
    $runs = @($batch | ForEach-Object { Start-LiveFireScenario $_ })
    foreach ($run in $runs) {
        $run.Process.WaitForExit()
    }
    foreach ($run in $runs) {
        $summary = Read-ScenarioSummary $run
        Add-Content -LiteralPath $summaryPath -Value ($summary | ConvertTo-Json -Compress -Depth 8) -Encoding utf8
        $processed++
        if (-not $summary.succeeded) { $failures++ }
        $totals.prompt_tokens += $summary.prompt_tokens
        $totals.cache_hit_tokens += $summary.prompt_cache_hit_tokens
        $totals.cache_miss_tokens += $summary.prompt_cache_miss_tokens
        $totals.completion_tokens += $summary.completion_tokens
        $totals.stages += $summary.stage_count
        $totals.attempts += $summary.provider_attempt_count
    }
    $currentBalance = Get-DeepSeekBalance
    $status = [ordered]@{
        schema = 'ghostlight.live_fire_matrix_status.v1'
        state = 'running'
        run_root = $resolvedRunRoot
        production_runtime_touched = $false
        target_balance = $TargetBalance
        starting_balance = $startingBalance
        current_balance = $currentBalance
        spent = $startingBalance - $currentBalance
        scenarios_processed = $processed
        scenarios_available = $scenarios.Count
        scenario_start_index = $StartAt
        scenario_catalog_count = $scenarioCatalogCount
        resume_run_roots = $resolvedResumeRoots
        previously_succeeded_skipped = $resumedScenarioIds.Count
        failures = $failures
        totals = $totals
        updated_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
    }
    $status | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statusPath -Encoding utf8
}

$state = if ($currentBalance -le $TargetBalance) { 'target_reached' } elseif ($processed -ge $scenarios.Count) { 'scenario_matrix_exhausted' } else { 'stopped' }
$finalStatus = [ordered]@{
    schema = 'ghostlight.live_fire_matrix_status.v1'
    state = $state
    run_root = $resolvedRunRoot
    production_runtime_touched = $false
    target_balance = $TargetBalance
    starting_balance = $startingBalance
    current_balance = $currentBalance
    spent = $startingBalance - $currentBalance
    scenarios_processed = $processed
    scenarios_available = $scenarios.Count
    scenario_start_index = $StartAt
    scenario_catalog_count = $scenarioCatalogCount
    resume_run_roots = $resolvedResumeRoots
    previously_succeeded_skipped = $resumedScenarioIds.Count
    failures = $failures
    totals = $totals
    summary_path = $summaryPath
    completed_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
}
$finalStatus | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $statusPath -Encoding utf8
$finalStatus
