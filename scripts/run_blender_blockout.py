"""Run Blender blockout generation with a stable local Blender path."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BLENDER = Path(r"C:\Program Files (x86)\Steam\steamapps\common\Blender\blender.exe")
BUILDER = ROOT / "scripts" / "blender" / "build_visual_blockout.py"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--blender", type=Path, default=DEFAULT_BLENDER)
    args, builder_args = parser.parse_known_args()

    blender = args.blender
    if not blender.exists():
        raise SystemExit(f"Blender executable not found: {blender}")
    if builder_args and builder_args[0] == "--":
        builder_args = builder_args[1:]
    if not builder_args:
        raise SystemExit("Pass build args after --, including --plan.")

    command = [str(blender), "--background", "--python", str(BUILDER), "--", *builder_args]
    print("running:", " ".join(f'\"{part}\"' if " " in part else part for part in command))
    return subprocess.call(command, cwd=ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
