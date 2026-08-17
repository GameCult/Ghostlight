[CmdletBinding()]
param(
    [string]$RunRoot = (Join-Path 'F:\GameCult\GhostlightDungeon\acceptance' ("http-journey-{0}-{1}" -f [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmss'), [guid]::NewGuid().ToString('N'))),
    [string]$SourceRoot = 'F:\Projects\Ghostlight',
    [ValidateRange(1024, 65535)]
    [int]$Port = 8841,
    [ValidateRange(30, 600)]
    [int]$RequestTimeoutSeconds = 240
)

$ErrorActionPreference = 'Stop'
$acceptanceRoot = [IO.Path]::GetFullPath('F:\GameCult\GhostlightDungeon\acceptance')
$resolvedRoot = [IO.Path]::GetFullPath($RunRoot)
if (-not $resolvedRoot.StartsWith($acceptanceRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "HTTP journey output must remain below $acceptanceRoot"
}
if (Test-Path -LiteralPath $resolvedRoot) {
    throw "HTTP journey output already exists: $resolvedRoot"
}
$binary = Join-Path $SourceRoot 'target\debug\ghostlight-dungeon.exe'
$secret = 'F:\GameCult\GhostlightDungeon\secrets\deepseek.dpapi'
foreach ($required in @($binary, $secret)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required journey input is absent: $required"
    }
}

$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, $Port)
try {
    $listener.Start()
} catch {
    throw "Loopback port $Port is already occupied"
} finally {
    $listener.Stop()
}

New-Item -ItemType Directory -Path $resolvedRoot, (Join-Path $resolvedRoot 'secrets'), (Join-Path $resolvedRoot 'boundary') | Out-Null
Copy-Item -LiteralPath $secret -Destination (Join-Path $resolvedRoot 'secrets\deepseek.dpapi')
$baseUri = "http://127.0.0.1:$Port"
$tokens = 1..2 | ForEach-Object {
    $bytes = [byte[]]::new(32)
    [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    try {
        [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
    } finally {
        [Array]::Clear($bytes, 0, $bytes.Length)
    }
}
$inviteMaterial = $tokens -join ','
$daemonGeneration = 0
$journeyStartedAt = [DateTimeOffset]::UtcNow

function Save-Json([string]$Name, $Value) {
    $path = Join-Path $resolvedRoot "boundary\$Name.json"
    [IO.File]::WriteAllText($path, ($Value | ConvertTo-Json -Depth 100))
    $path
}

function Start-JourneyDaemon {
    $script:daemonGeneration++
    $stdout = Join-Path $resolvedRoot ("daemon-{0}.stdout.log" -f $script:daemonGeneration)
    $stderr = Join-Path $resolvedRoot ("daemon-{0}.stderr.log" -f $script:daemonGeneration)
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $binary
    $psi.WorkingDirectory = $SourceRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment['GHOSTLIGHT_DUNGEON_ROOT'] = $resolvedRoot
    $psi.Environment['GHOSTLIGHT_DUNGEON_BIND'] = "127.0.0.1:$Port"
    $psi.Environment['GHOSTLIGHT_INVITES'] = $inviteMaterial
    $process = [Diagnostics.Process]::Start($psi)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    [pscustomobject]@{
        Process = $process
        StdoutTask = $stdoutTask
        StderrTask = $stderrTask
        Stdout = $stdout
        Stderr = $stderr
    }
}

function Stop-JourneyDaemon($Daemon) {
    if (-not $Daemon.Process.HasExited) {
        $Daemon.Process.Kill($true)
        $Daemon.Process.WaitForExit(10000)
    }
    [IO.File]::WriteAllText($Daemon.Stdout, $Daemon.StdoutTask.GetAwaiter().GetResult())
    [IO.File]::WriteAllText($Daemon.Stderr, $Daemon.StderrTask.GetAwaiter().GetResult())
}

function Wait-JourneyHealth($Daemon) {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($RequestTimeoutSeconds)
    do {
        if ($Daemon.Process.HasExited) {
            Stop-JourneyDaemon $Daemon
            throw "Journey daemon exited during startup; inspect $($Daemon.Stderr)"
        }
        try {
            $response = Invoke-WebRequest -Uri "$baseUri/health" -TimeoutSec 5
            if ($response.StatusCode -eq 200) {
                return $response.Content | ConvertFrom-Json -Depth 100
            }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    } while ([DateTimeOffset]::UtcNow -lt $deadline)
    throw "Journey daemon did not become healthy within $RequestTimeoutSeconds seconds"
}

function Invoke-JourneyRequest {
    param(
        [Parameter(Mandatory)][ValidateSet('GET', 'POST')][string]$Method,
        [Parameter(Mandatory)][string]$Path,
        [Microsoft.PowerShell.Commands.WebRequestSession]$Session,
        $Body,
        [string]$OutFile
    )
    $parameters = @{
        Uri = "$baseUri$Path"
        Method = $Method
        TimeoutSec = $RequestTimeoutSeconds
    }
    if ($Session) { $parameters.WebSession = $Session }
    if ($null -ne $Body) {
        $parameters.ContentType = 'application/json'
        $parameters.Body = $Body | ConvertTo-Json -Depth 100 -Compress
    }
    if ($OutFile) { $parameters.OutFile = $OutFile }
    $response = Invoke-WebRequest @parameters
    if ($OutFile) { return $response }
    if ([string]::IsNullOrWhiteSpace($response.Content)) { return $null }
    if ($response.Headers.'Content-Type' -like '*json*') {
        return $response.Content | ConvertFrom-Json -Depth 100
    }
    $response.Content
}

function Get-SelectedCampaign($Session) {
    $campaigns = Invoke-JourneyRequest -Method GET -Path '/api/campaigns' -Session $Session
    $selected = @($campaigns.campaigns | Where-Object selected)
    if ($selected.Count -ne 1) {
        throw "Expected one selected campaign, found $($selected.Count)"
    }
    $selected[0]
}

function Find-EveNode($Node, [string]$Id) {
    if ($null -eq $Node) { return $null }
    if ($Node.id -eq $Id) { return $Node }
    foreach ($child in @($Node.children)) {
        $found = Find-EveNode $child $Id
        if ($found) { return $found }
    }
    $null
}

$daemon = $null
try {
    $daemon = Start-JourneyDaemon
    $healthBefore = Wait-JourneyHealth $daemon
    Save-Json 'health-before' $healthBefore | Out-Null

    $unauthorized = Invoke-WebRequest -Uri "$baseUri/api/surface" -SkipHttpErrorCheck
    if ($unauthorized.StatusCode -ne 401) {
        throw "Unauthenticated surface returned $($unauthorized.StatusCode), expected 401"
    }

    $testerOne = [Microsoft.PowerShell.Commands.WebRequestSession]::new()
    $testerTwo = [Microsoft.PowerShell.Commands.WebRequestSession]::new()
    Invoke-WebRequest -Uri "$baseUri/invite/$($tokens[0])" -WebSession $testerOne | Out-Null
    Invoke-WebRequest -Uri "$baseUri/invite/$($tokens[1])" -WebSession $testerTwo | Out-Null
    $consumed = Invoke-WebRequest -Uri "$baseUri/invite/$($tokens[0])" -SkipHttpErrorCheck
    if ($consumed.StatusCode -ne 401) {
        throw "Consumed invite returned $($consumed.StatusCode), expected 401"
    }
    $testerTwoInitial = Invoke-JourneyRequest -Method GET -Path '/api/campaigns' -Session $testerTwo
    if (@($testerTwoInitial.campaigns).Count -ne 0) {
        throw 'Second tester inherited another session campaign'
    }

    $compile = Invoke-JourneyRequest -Method POST -Path '/api/compiler/custom' -Session $testerOne -Body @{
        campaign_name = 'The Embargo Ledger'
        who = 'a junior Lucent maintenance runner trained to visually inspect public equipment, carrying no tools and holding no institutional authority'
        where_ = 'a Lucent newsroom during an evidence embargo'
        when = 'the narrowest era the retrieved Aetheria evidence can honestly support'
        goal = 'verify one publicly observable inconsistency without taking custody of a source or inventing authority'
    }
    Save-Json 'compile-preview' $compile | Out-Null
    if (-not $compile.preview_id -or $compile.preview.locations.Count -lt 1) {
        throw 'Compiler returned no approvable bounded world'
    }
    $approve = Invoke-JourneyRequest -Method POST -Path "/api/compiler/approve/$($compile.preview_id)" -Session $testerOne
    Save-Json 'approve' $approve | Out-Null
    $original = Get-SelectedCampaign $testerOne

    $forbidden = Invoke-WebRequest -Uri "$baseUri/api/campaigns/select/$($original.id)" -Method POST -WebSession $testerTwo -SkipHttpErrorCheck
    if ($forbidden.StatusCode -ne 403) {
        throw "Second tester selected the first tester's campaign with status $($forbidden.StatusCode)"
    }

    $surfaceBefore = Invoke-JourneyRequest -Method GET -Path '/api/surface' -Session $testerOne
    Save-Json 'surface-before-turn' $surfaceBefore | Out-Null
    $speak = Invoke-JourneyRequest -Method POST -Path '/api/command' -Session $testerOne -Body @{
        type = 'speak'
        expected_revision = [uint64]$surfaceBefore.world_revision
        actor_id = 'player'
        text = 'I set my empty hands where everyone can see them. I am not asking for trust. Which part of this embargo can a runner verify without taking custody of your sources?'
        intended_effect = 'invite a bounded answer without compelling disclosure or claiming authority'
    }
    Save-Json 'speak' $speak | Out-Null

    $afterSpeak = Get-SelectedCampaign $testerOne
    $impossible = Invoke-JourneyRequest -Method POST -Path '/api/command' -Session $testerOne -Body @{
        type = 'assess'
        expected_revision = [uint64]$afterSpeak.revision
        intent = @{
            actor_id = 'player'
            description = 'Teleport into a nonexistent sealed archive and declare every Lucent source legally mine.'
            intended_effect = 'gain remote access, custody, ownership, and institutional authority immediately'
        }
    }
    Save-Json 'assessment-impossible' $impossible | Out-Null
    if ($impossible.kind -ne 'assessed' -or $impossible.assessment.admissible) {
        throw 'Impossible overreach was admitted to a roll'
    }
    $afterImpossible = Get-SelectedCampaign $testerOne
    if ($afterImpossible.revision -ne $afterSpeak.revision) {
        throw 'Private assessment changed campaign revision or became visible world state'
    }

    $surfaceForAction = Invoke-JourneyRequest -Method GET -Path '/api/surface' -Session $testerOne
    $ledger = Find-EveNode $surfaceForAction.surface.root 'dungeon.ledger.text'
    $capabilityLine = ([string]$ledger.props.value -split "`n" | Where-Object { $_ -like 'Capabilities:*' } | Select-Object -First 1)
    $capability = ($capabilityLine -replace '^Capabilities:\s*', '' -split ',' | Select-Object -First 1).Trim()
    if ([string]::IsNullOrWhiteSpace($capability) -or $capability -eq 'none') {
        throw 'Compiled player role exposed no capability for the bounded action test'
    }
    $plausible = Invoke-JourneyRequest -Method POST -Path '/api/command' -Session $testerOne -Body @{
        type = 'assess'
        expected_revision = [uint64]$afterImpossible.revision
        intent = @{
            actor_id = 'player'
            description = "Use my existing capability exactly as recorded: $capability. I act only from my current position and touch nothing I do not possess."
            intended_effect = 'produce the smallest directly observable result that this existing capability, current location, and current access permit'
        }
    }
    Save-Json 'assessment-plausible' $plausible | Out-Null
    if ($plausible.kind -ne 'assessed' -or -not $plausible.assessment.admissible) {
        throw 'A bounded use of an existing capability was not admitted to a roll'
    }
    $attempt = Invoke-JourneyRequest -Method POST -Path '/api/command' -Session $testerOne -Body @{
        type = 'attempt'
        assessment_digest = $plausible.assessment.digest
    }
    Save-Json 'attempt' $attempt | Out-Null
    if ($attempt.kind -ne 'committed' -or -not $attempt.receipt.roll) {
        throw 'Confirmed assessment did not atomically commit a server roll'
    }

    $afterAttempt = Get-SelectedCampaign $testerOne
    $wait = Invoke-JourneyRequest -Method POST -Path '/api/command' -Session $testerOne -Body @{
        type = 'wait'
        expected_revision = [uint64]$afterAttempt.revision
        minutes = 60
    }
    Save-Json 'wait' $wait | Out-Null
    $beforeFork = Get-SelectedCampaign $testerOne
    $surfaceAfter = Invoke-JourneyRequest -Method GET -Path '/api/surface' -Session $testerOne
    Save-Json 'surface-after-turns' $surfaceAfter | Out-Null
    $operatorAfter = Invoke-JourneyRequest -Method GET -Path '/api/operator' -Session $testerOne
    Save-Json 'operator-after-turns' $operatorAfter | Out-Null
    $operatorTypedNode = Find-EveNode $operatorAfter.surface.root 'dungeon.operator.typed'
    if (-not $operatorTypedNode) {
        throw 'Operator surface omitted its typed inspector projection'
    }
    $operatorTyped = [string]$operatorTypedNode.props.value | ConvertFrom-Json -Depth 100

    $fork = Invoke-JourneyRequest -Method POST -Path '/api/campaigns/fork' -Session $testerOne -Body @{ name = 'The Embargo Ledger — Fork' }
    Save-Json 'fork' $fork | Out-Null
    if ($fork.kind -ne 'forked' -or $fork.revision -ne 0) {
        throw 'Campaign fork did not begin as an isolated revision-zero branch'
    }
    $exportPath = Join-Path $resolvedRoot 'fork-export.cc'
    Invoke-JourneyRequest -Method GET -Path '/api/campaigns/export' -Session $testerOne -OutFile $exportPath | Out-Null
    if ((Get-Item -LiteralPath $exportPath).Length -le 0) {
        throw 'Campaign export is empty'
    }

    Stop-JourneyDaemon $daemon
    $daemon = Start-JourneyDaemon
    $healthAfter = Wait-JourneyHealth $daemon
    Save-Json 'health-after-restart' $healthAfter | Out-Null
    $reloadedCampaigns = Invoke-JourneyRequest -Method GET -Path '/api/campaigns' -Session $testerOne
    Save-Json 'campaigns-after-restart' $reloadedCampaigns | Out-Null
    if (@($reloadedCampaigns.campaigns).Count -ne 2) {
        throw 'Daemon restart did not reload both owned campaign branches'
    }
    Invoke-JourneyRequest -Method POST -Path "/api/campaigns/select/$($original.id)" -Session $testerOne | Out-Null
    $reloadedOriginal = Get-SelectedCampaign $testerOne
    if ($reloadedOriginal.revision -ne $beforeFork.revision) {
        throw 'Original campaign revision changed across fork or daemon restart'
    }
    $reloadedSurface = Invoke-JourneyRequest -Method GET -Path '/api/surface' -Session $testerOne
    Save-Json 'surface-after-restart' $reloadedSurface | Out-Null
    $testerTwoAfter = Invoke-JourneyRequest -Method GET -Path '/api/campaigns' -Session $testerTwo
    if (@($testerTwoAfter.campaigns).Count -ne 0) {
        throw 'Second tester acquired the first tester campaign after restart'
    }

    $story = Find-EveNode $reloadedSurface.surface.root 'dungeon.transcript'
    $narrations = @($story.children | Where-Object id -like 'narration-*')
    $transcript = @($story.children | Where-Object id -like 'turn-*')
    $storyRevisions = @($story.children | ForEach-Object {
        if ([string]$_.id -match '^(?:narration|turn)-(\d+)') { [uint64]$Matches[1] }
    })
    for ($index = 1; $index -lt $storyRevisions.Count; $index++) {
        if ($storyRevisions[$index] -lt $storyRevisions[$index - 1]) {
            throw 'Player story projection is not chronological by campaign revision'
        }
    }
    $stageReceipts = @($operatorTyped.model_stage_receipts)
    $providerAttempts = @($stageReceipts | ForEach-Object { @($_.provider_attempts) })
    $tokenUsage = @($providerAttempts | ForEach-Object { $_.token_usage } | Where-Object { $null -ne $_ })
    $promptTokens = [uint64](($tokenUsage | Measure-Object prompt_tokens -Sum).Sum ?? 0)
    $cacheHitTokens = [uint64](($tokenUsage | Measure-Object prompt_cache_hit_tokens -Sum).Sum ?? 0)
    $completionTokens = [uint64](($tokenUsage | Measure-Object completion_tokens -Sum).Sum ?? 0)
    $stageRollup = @($stageReceipts | Group-Object stage | ForEach-Object {
        $groupAttempts = @($_.Group | ForEach-Object { @($_.provider_attempts) })
        $groupUsage = @($groupAttempts | ForEach-Object { $_.token_usage } | Where-Object { $null -ne $_ })
        [ordered]@{
            stage = $_.Name
            receipts = $_.Count
            attempts = $groupAttempts.Count
            invalid_receipts = @($_.Group | Where-Object validation_result -ne 'valid').Count
            prompt_tokens = [uint64](($groupUsage | Measure-Object prompt_tokens -Sum).Sum ?? 0)
            cache_hit_tokens = [uint64](($groupUsage | Measure-Object prompt_cache_hit_tokens -Sum).Sum ?? 0)
            completion_tokens = [uint64](($groupUsage | Measure-Object completion_tokens -Sum).Sum ?? 0)
        }
    })
    $commit = (& git -C $SourceRoot rev-parse HEAD).Trim()
    $summary = [ordered]@{
        schema = 'ghostlight.http_journey_smoke.v1'
        source_commit = $commit
        production_runtime_touched = $false
        listener = "127.0.0.1:$Port"
        elapsed_seconds = ([DateTimeOffset]::UtcNow - $journeyStartedAt).TotalSeconds
        startup_health = $healthBefore.status
        restart_health = $healthAfter.status
        unauthenticated_status = [int]$unauthorized.StatusCode
        consumed_invite_status = [int]$consumed.StatusCode
        cross_session_select_status = [int]$forbidden.StatusCode
        campaign_id = $original.id
        campaign_revision_after_journey = [uint64]$reloadedOriginal.revision
        fork_campaign_id = $fork.campaign_id
        reloaded_owned_campaigns = @($reloadedCampaigns.campaigns).Count
        second_tester_campaigns = @($testerTwoAfter.campaigns).Count
        compiled_locations = @($compile.preview.locations).Count
        compiled_cast = @($compile.preview.cast).Count + 1
        compiled_institutions = @($compile.preview.institutions).Count
        compile_gaps = @($compile.preview.gaps).Count
        impossible_admissible = [bool]$impossible.assessment.admissible
        impossible_bargains = @($impossible.assessment.bargains).Count
        plausible_capability = $capability
        plausible_dc = [int]$plausible.assessment.dc
        roll = $attempt.receipt.roll
        narration_count = $narrations.Count
        transcript_count = $transcript.Count
        story_revisions = $storyRevisions
        world_commit_count = @($operatorTyped.commit_receipts).Count
        model_stage_count = $stageReceipts.Count
        provider_attempt_count = $providerAttempts.Count
        prompt_tokens = $promptTokens
        cache_hit_tokens = $cacheHitTokens
        cache_hit_ratio = if ($promptTokens -gt 0) { $cacheHitTokens / $promptTokens } else { 0 }
        completion_tokens = $completionTokens
        stage_rollup = $stageRollup
        export_bytes = (Get-Item -LiteralPath $exportPath).Length
        evidence_paths = Get-ChildItem -LiteralPath (Join-Path $resolvedRoot 'boundary') -File | Select-Object -ExpandProperty FullName
    }
    Save-Json 'summary' $summary | Out-Null
    $summary | ConvertTo-Json -Depth 20
} finally {
    if ($daemon) {
        Stop-JourneyDaemon $daemon
    }
    [Array]::Clear($tokens, 0, $tokens.Length)
    $inviteMaterial = $null
}
