# Ghostlight Scripts

## Visual Image Inventory

`count_visual_images.py` counts base images and branch/state variants declared by
Ghostlight `.visual.json` plans. Use it before committing to an illustrated IF
fixture so the render burden is visible instead of quietly breeding in a vent.

Run the default inventory:

```powershell
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' .\scripts\count_visual_images.py
```

Run with per-scene detail:

```powershell
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' .\scripts\count_visual_images.py --verbose
```

## BFL Image Batches

`generate_bfl_images.py` submits a compact prompt manifest to the BFL async
image API, polls the returned status URL, downloads signed results immediately,
and writes a local run manifest. Generated images belong under
`experiments/images/`, which is ignored by git.

Run the Corvid simple-prompt batch:

```powershell
& 'C:\Users\Meta\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' .\scripts\generate_bfl_images.py .\prompts\image-generation\corvid-collective-founding.bfl-simple-prompts.v0.json --out-dir .\experiments\images\corvid-collective-founding-v0\bfl --skip-existing --continue-on-error
```
