#!/usr/bin/env python3
"""
Fix WASM paths for Dioxus production builds.

Dioxus 0.7 hashes WASM/JS files and places them in assets/, but index.html
still references the old wasm/ paths. This script creates symlinks to bridge
the gap.
"""

import json
import os
import sys


def main():
    manifest_path = "target/dx/yggdrasil/release/web/.manifest.json"

    if not os.path.exists(manifest_path):
        print("Manifest not found, skipping WASM path fix")
        sys.exit(0)

    with open(manifest_path) as f:
        data = json.load(f)

    wasm_dir = "target/dx/yggdrasil/release/web/public/wasm"
    assets_dir = "target/dx/yggdrasil/release/web/public/assets"
    os.makedirs(wasm_dir, exist_ok=True)

    for path, info_list in data["assets"].items():
        if "wasm/" in path:
            for info in info_list:
                bundled = info["bundled_path"]
                original_name = os.path.basename(path)
                src = os.path.join(assets_dir, bundled)
                dst = os.path.join(wasm_dir, original_name)

                if os.path.exists(src):
                    if os.path.islink(dst) or os.path.exists(dst):
                        os.remove(dst)
                    os.symlink(os.path.relpath(src, os.path.dirname(dst)), dst)
                    print(f"Linked: wasm/{original_name} -> assets/{bundled}")
                else:
                    print(f"ERROR: Source not found: {src}")
                    sys.exit(1)


if __name__ == "__main__":
    main()
