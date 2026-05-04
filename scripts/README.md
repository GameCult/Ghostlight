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
