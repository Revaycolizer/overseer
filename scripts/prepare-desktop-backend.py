#!/usr/bin/env python3
"""Build and stage the native daemon for Tauri resource bundling."""
import os, pathlib, shutil, subprocess
root=pathlib.Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','--locked','--release','--bin','overseer'],cwd=root,check=True)
name='overseer.exe' if os.name=='nt' else 'overseer'
folder=root/'desktop-ui/src-tauri/binaries';folder.mkdir(exist_ok=True)
shutil.copy2(root/'target/release'/name,folder/name)
