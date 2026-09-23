$root = 'C:\Meta\ghostlight'
# The daemon's own default runtime root on Windows is F:\GameCult\GhostlightDungeon.
# --state-root cannot validate on Windows: fs::canonicalize returns a \?\ path
# that never equals the argument, so the check always fails (recorded as a finding).
$env:GHOSTLIGHT_RUNTIME_ID                    = 'ghostlight-dungeon-raven'
$env:GHOSTLIGHT_HEIMDALL_APP_SECRET_FILE      = "$root\state\secrets\heimdall-app.secret"
$env:GHOSTLIGHT_SESSION_WRAPPING_KEY_FILE     = "$root\state\secrets\session-wrapping.key"
$env:GHOSTLIGHT_ODIN_RUDP                     = '10.77.0.1:17871'
$env:GHOSTLIGHT_DISCORD_GUILD_ID              = '113786069023064068'
$env:GHOSTLIGHT_DISCORD_ROLE_ID               = '861962027550244914'
$env:GHOSTLIGHT_PUBLIC_APP_URL                = 'http://127.0.0.1:8831/'
$env:GHOSTLIGHT_LOCAL_ENDPOINT                = '127.0.0.1:8080'
$env:GHOSTLIGHT_CONTROLLER_PROJECTOR_MODEL    = 'local/bonsai-2-27b'
$env:GHOSTLIGHT_CONTROLLER_PERSONA_MODEL      = 'local/bonsai-2-27b'
$env:GHOSTLIGHT_CONTROLLER_INTERPRETER_MODEL  = 'local/bonsai-2-27b'
$env:GHOSTLIGHT_CONTROLLER_OPERATIONAL_MODEL  = 'local/bonsai-2-27b'
$env:GHOSTLIGHT_CONTROLLER_ELABORATOR_MODEL   = 'local/bonsai-2-27b'
$env:GHOSTLIGHT_PLAY_MODEL                    = 'local/bonsai-2-27b'
$env:RUST_LOG                                 = 'info'
$env:RUST_BACKTRACE                           = '1'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
New-Item -ItemType Directory -Force -Path "$root\logs" | Out-Null
& "$root\ghostlight-dungeon.exe" *>> "$root\logs\daemon-$stamp.log"

# Bring-up, 2026-09-23 (playtest, not a deployment path -- Idunn owns deploys):
#   1. Bonsai 2 must be up in WSL on Raven: bash /opt/gamecult/bonsai/run.sh
#      Stop bonsai-idle.timer while playing; its age check misreads the skewed
#      WSL clock and stops the model under a live session. Restore afterwards.
#   2. Secrets live beside this script, under state\secrets\: the Heimdall app
#      secret (copied from Yggdrasil) and a session wrapping key generated here.
#   3. The runtime id ghostlight-dungeon-raven must be in Heimdall's
#      GC_ACCESS_APP_GHOSTLIGHT_RUNTIME_IDS on Yggdrasil.
#   4. Registered as a scheduled task so it survives the SSH session that
#      started it:
#        Register-ScheduledTask -TaskName GhostlightDungeon -Action (
#          New-ScheduledTaskAction -Execute powershell.exe -Argument
#          '-NoProfile -ExecutionPolicy Bypass -File C:\Meta\ghostlight\run-svc.ps1')
#   5. The UI is served from <exe dir>\web, built with: npm --prefix web run build
#
# No --state-root: the runtime root is the Windows default,
# F:\GameCult\GhostlightDungeon. --state-root cannot validate on Windows
# because fs::canonicalize returns a \?\ path that never equals the argument.
