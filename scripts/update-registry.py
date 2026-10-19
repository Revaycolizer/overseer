#!/usr/bin/env python3
import json
import sys
import os
from datetime import datetime

def main():
    if len(sys.argv) < 2:
        print("Usage: update-registry.py <manifest_path> [summary]")
        sys.exit(1)

    manifest_path = sys.argv[1]
    summary = sys.argv[2] if len(sys.argv) > 2 else None
    registry_path = "releases/releases.json"

    if not os.path.exists(manifest_path):
        print(f"Manifest not found: {manifest_path}")
        sys.exit(1)

    with open(manifest_path, 'r') as f:
        manifest = json.load(f)

    if os.path.exists(registry_path):
        with open(registry_path, 'r') as f:
            registry = json.load(f)
    else:
        registry = {
            "product": manifest["product"],
            "updated_at": "",
            "latest": "",
            "channels": {},
            "badges": [],
            "releases": []
        }

    version = manifest["version"]
    tag = manifest["tag"]
    platform = manifest["platform"]
    channel = manifest.get("channel", "stable")

    # Find or create release entry
    release = next((r for r in registry["releases"] if r["version"] == version), None)
    if not release:
        release = {
            "version": version,
            "tag": tag,
            "channel": channel,
            "is_latest": False,
            "is_stable": channel == "stable",
            "labels": [],
            "published_at": manifest["built_at"],
            "summary": summary[:300] if summary else f"Automated release for {version}",
            "assets": []
        }
        registry["releases"].insert(0, release)
    elif summary:
        release["summary"] = summary[:300]

    # Update assets for this platform
    # Remove old assets for the same platform to avoid duplicates
    release["assets"] = [a for a in release["assets"] if a.get("platform") != platform]
    
    for asset in manifest["assets"]:
        # Map kind to label if possible
        label = asset["name"]
        if asset["kind"] == "appimage":
            label = "Linux AppImage"
        elif asset["kind"] == "portable-tarball":
            label = "Linux Portable Tarball"
        elif asset["kind"] == "setup-exe":
            label = "Windows Installer"
        elif asset["kind"] == "portable-zip":
            label = "Windows Portable Zip"

        release["assets"].append({
            "name": asset["name"],
            "platform": platform,
            "kind": asset["kind"],
            "label": label,
            "path": f"dist/{tag}/{asset['name']}"
        })

    # Update labels
    platforms = sorted(list(set(a["platform"] for a in release["assets"])))
    release["labels"] = platforms + [release["channel"]]
    if version == registry.get("latest"):
        if "latest" not in release["labels"]:
            release["labels"].append("latest")

    # Sort releases by version (naive descending)
    registry["releases"].sort(key=lambda r: r["version"], reverse=True)

    # Update global pointers
    if registry["releases"]:
        latest_release = registry["releases"][0]
        registry["latest"] = latest_release["tag"]
        registry["updated_at"] = datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
        
        for r in registry["releases"]:
            r["is_latest"] = (r["tag"] == registry["latest"])

        # Update channels
        registry["channels"]["latest"] = registry["latest"]
        if channel == "stable":
            registry["channels"]["stable"] = tag

    # Save registry
    with open(registry_path, 'w') as f:
        json.dump(registry, f, indent=2)

    print(f"Successfully updated registry with {platform} assets for {version}")

if __name__ == "__main__":
    main()
