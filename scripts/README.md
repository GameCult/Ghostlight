# Ghostlight Scripts

## BFL Image Generation

`generate_bfl_images.py` submits prompts to the Black Forest Labs async image
API, polls the returned `polling_url`, downloads the signed `result.sample`
image immediately, and writes a metadata receipt beside the image.

For Ghostlight visual-plan modifiers, the script uses the durable continuity
workflow:

1. Generate the scene base image from the assembled base prompt.
2. Save the base image locally.
3. Send that base image back as `input_image`.
4. Send only the selected modifier prompt as the edit instruction.

Dry-runs write `.base.prompt.txt` and, when modifiers are selected,
`.edit.prompt.txt` separately so the two prompts can be inspected before
credits get thrown into the machine.

The script reads the API key from `BFL_API_KEY` first. If that is not set, it
falls back to `E:\Projects\gamecult-ops\bfl-api.txt`. It never prints the key.

Dry-run a direct prompt:

```powershell
npm run images:bfl -- --prompt "A maintenance yard in a rotating asteroid habitat" --dry-run
```

Dry-run one Ghostlight visual scene prompt:

```powershell
npm run images:bfl -- --visual-plan examples\visual\pallas-species-strikes.branch-and-fold.v0.visual.json --scene-id pallas_yard_wake --dry-run
```

Dry-run a base scene plus one edit modifier:

```powershell
npm run images:bfl -- --visual-plan examples\visual\pallas-species-strikes.branch-and-fold.v0.visual.json --scene-id pallas_ilya_arrival --global-modifier-id formal_stoppage_visible --dry-run
```

Submit one paid render:

```powershell
npm run images:bfl -- --visual-plan examples\visual\pallas-species-strikes.branch-and-fold.v0.visual.json --scene-id pallas_yard_wake --width 1536 --height 864 --output-format png --yes
```

If modifier ids are supplied, one visual scene costs two BFL jobs: one base
generation and one edit pass from that base image.

Render every base scene from a visual plan:

```powershell
npm run images:bfl -- --visual-plan examples\visual\pallas-species-strikes.branch-and-fold.v0.visual.json --all-scenes --width 1536 --height 864 --output-format png --yes
```

Generated images and receipts go under `experiments/images/`, which is ignored
by git. Move curated artifacts elsewhere deliberately if they need to become
project assets.
