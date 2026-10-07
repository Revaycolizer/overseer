#!/usr/bin/env python3
"""Smoke-test local Mac release artifacts without using the normal user database.

Run after building the engine, terminal, and desktop application. No AI request,
desktop input automation, microphone recording, or external service is required.
"""

import argparse
import base64
import fcntl
import hashlib
import json
import os
from pathlib import Path
import plistlib
import pty
import re
import select
import signal
import socket
import sqlite3
import struct
import subprocess
import tempfile
import termios
import time
import urllib.error
import urllib.request


def stop(process):
    if process.poll() is None:
        process.send_signal(signal.SIGINT)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def read_terminal(fd, timeout):
    output = bytearray()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        ready, _, _ = select.select([fd], [], [], max(0, min(0.1, deadline - time.monotonic())))
        if ready:
            try:
                output.extend(os.read(fd, 65536))
            except OSError:
                break
    return bytes(output)


def send_ws_json(ws, payload):
    body = json.dumps(payload).encode()
    mask = os.urandom(4)
    header = bytes([0x81, 0x80 | len(body)]) if len(body) < 126 else bytes([0x81, 0xfe]) + struct.pack("!H", len(body))
    ws.sendall(header + mask + bytes(value ^ mask[index % 4] for index, value in enumerate(body)))


def receive_ws_json(ws):
    def read(count):
        result = b""
        while len(result) < count:
            chunk = ws.recv(count - len(result))
            if not chunk:
                raise RuntimeError("WebSocket disconnected")
            result += chunk
        return result
    first, second = read(2)
    if first & 0x0f != 1:
        raise RuntimeError(f"Expected a text frame: {first}")
    length = second & 0x7f
    if length == 126:
        length = struct.unpack("!H", read(2))[0]
    elif length == 127:
        length = struct.unpack("!Q", read(8))[0]
    if length > 1024 * 1024:
        raise RuntimeError("Unexpectedly large WebSocket response")
    return json.loads(read(length))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    root = args.root.resolve()
    daemon = root / "target/release/overseer"
    terminal = root / "target/release/overseer-tui"
    app = root / "desktop-ui/src-tauri/target/release/bundle/macos/Overseer.app"
    with (app / "Contents/Info.plist").open("rb") as handle:
        metadata = plistlib.load(handle)
    desktop = app / "Contents/MacOS" / metadata["CFBundleExecutable"]
    for artifact in (daemon, terminal, desktop):
        if not artifact.is_file():
            raise RuntimeError(f"Missing build artifact: {artifact}")

    # Allocate a free port so a resident user daemon is never stopped or tested.
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    report = {"platform": "macOS", "checks": {}, "artifacts": {}}
    for artifact in (daemon, terminal, desktop):
        with artifact.open("rb") as handle:
            digest = hashlib.file_digest(handle, "sha256").hexdigest()
        report["artifacts"][str(artifact.relative_to(root))] = {
            "bytes": artifact.stat().st_size,
            "sha256": digest,
        }
    report["checks"]["desktop_bundle"] = metadata["CFBundleIdentifier"]
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    report["checks"]["desktop_code_signature"] = "passed"

    with tempfile.TemporaryDirectory(prefix="overseer-verify-") as scratch:
        env = dict(os.environ, OVERSEER_DATA_DIR=scratch, OVERSEER_WAKE="off")
        env.pop("OVERSEER_API_TOKEN", None)
        with (Path(scratch) / "daemon.log").open("w+") as log:
            process = subprocess.Popen([str(daemon), "--server", "--port", str(port)], cwd=root, env=env,
                                       stdout=log, stderr=subprocess.STDOUT)
            try:
                # Bypass HTTP proxy environment variables for this local probe.
                http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                deadline = time.monotonic() + 45
                while True:
                    if process.poll() is not None:
                        log.seek(0)
                        raise RuntimeError(f"Daemon exited early:\n{log.read()}")
                    try:
                        with http.open(f"http://127.0.0.1:{port}/healthz", timeout=4) as response:
                            health = json.load(response)
                        if health.get("status") != "healthy" or not health.get("db_connected"):
                            raise RuntimeError(f"Unhealthy daemon: {health}")
                        break
                    except (urllib.error.URLError, TimeoutError):
                        if time.monotonic() >= deadline:
                            log.seek(0)
                            raise RuntimeError(f"Daemon startup timed out:\n{log.read()}")
                        time.sleep(0.2)
                if health.get("uptime_secs", 10**9) > 30:
                    raise RuntimeError(f"Health uptime is not measured from startup: {health}")
                report["checks"]["daemon_health"] = health
                request = urllib.request.Request(f"http://127.0.0.1:{port}/api/models/available", headers={"Origin":"https://evil.example"})
                try:
                    http.open(request, timeout=5)
                    raise RuntimeError("Cross-site API request was accepted")
                except urllib.error.HTTPError as error:
                    if error.code != 403: raise
                report["checks"]["cross_site_protection"] = "passed"

                with socket.create_connection(("127.0.0.1", port), timeout=5) as ws:
                    key = base64.b64encode(os.urandom(16)).decode()
                    request = (
                        f"GET /api/ws/chat HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
                        "Upgrade: websocket\r\nConnection: Upgrade\r\n"
                        f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
                    )
                    ws.sendall(request.encode())
                    status = ws.recv(4096).split(b"\r\n", 1)[0]
                    if b" 101 " not in status:
                        raise RuntimeError(f"WebSocket handshake failed: {status!r}")
                    send_ws_json(ws, {"conversation_id":"0", "message":"/memory list"})
                    session = receive_ws_json(ws)
                    done = receive_ws_json(ws)
                    if session.get("type") != "session" or not isinstance(session.get("conversation_id"), str):
                        raise RuntimeError(f"Conversation session did not preserve the ID: {session}")
                    if done.get("type") != "done" or done.get("response") != "No saved memories.":
                        raise RuntimeError(f"Desktop WebSocket did not reach learning memory: {done}")
                report["checks"]["chat_websocket_upgrade"] = "passed"
                report["checks"]["desktop_websocket_memory_and_exact_ids"] = "passed"

                def post_json(path, payload):
                    request = urllib.request.Request(f"http://127.0.0.1:{port}" + path,
                        data=json.dumps(payload).encode(), headers={"Content-Type": "application/json"})
                    with http.open(request, timeout=10) as response:
                        return response.read().decode()
                conversation = json.loads(post_json("/api/conversations", {"title": "Memory verification"}))
                def memory_command(query):
                    return post_json("/api/orchestrate", {"conversation_id": conversation, "query": query})
                if "Remembered" not in memory_command("/memory remember style=concise answers"):
                    raise RuntimeError("Memory preference was not saved through the daemon")
                memories = memory_command("/memory list")
                memory_id = re.search(r"#(\d+)", memories)
                if not memory_id or "concise answers" not in memories:
                    raise RuntimeError("Memory inspection returned unexpected records")
                if "concise answers" not in memory_command(f"/memory show {memory_id.group(1)}"):
                    raise RuntimeError("Full memory inspection failed")
                if "paused" not in memory_command("/memory pause"):
                    raise RuntimeError("Memory pause failed")
                memory_command("/memory resume")
                memory_command(f"/memory forget {memory_id.group(1)}")
                if memory_command("/memory list") != "No saved memories.":
                    raise RuntimeError("Memory deletion failed")
                project = Path(scratch) / "project"
                project.mkdir()
                (project / "README.md").write_text("This rocket project builds using cargo test.")
                if "Indexed 1 documents" not in memory_command(f"/memory project {project}"):
                    raise RuntimeError("Project knowledge indexing failed")
                if "rocket project" not in memory_command("/memory list"):
                    raise RuntimeError("Project knowledge was not inspectable")
                memory_command("/memory clear")
                if memory_command("/memory list") != "No saved memories.":
                    raise RuntimeError("Memory clear failed")
                report["checks"]["persistent_memory_commands"] = "save, inspect, project index, pause, resume, delete and clear passed"

                master, slave = pty.openpty()
                tui = None
                try:
                    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
                    tui = subprocess.Popen(
                        [str(terminal), "--remote", "--url", f"http://127.0.0.1:{port}"],
                        cwd=root, env=dict(env, TERM="xterm-256color"),
                        stdin=slave, stdout=slave, stderr=slave,
                    )
                    os.close(slave)
                    slave = None
                    output = read_terminal(master, 3)
                    if tui.poll() is not None or b"Overseer" not in output:
                        raise RuntimeError(f"Terminal failed to start: {output.decode(errors='replace')}")
                    with http.open(f"http://127.0.0.1:{port}/api/models/available", timeout=5) as response:
                        models = json.load(response)
                    os.write(master, b"\x1b[17~")  # F6: model picker
                    picker = read_terminal(master, 1)
                    if b"Installed models" not in picker:
                        raise RuntimeError("F6 did not open the model picker")
                    if models:
                        expected = sorted(set(models))[0]
                        if expected.encode() not in picker:
                            raise RuntimeError(f"Installed model is missing from picker: {expected}")
                        os.write(master, b"\r")
                        read_terminal(master, 1)
                        for setting in ("default_chat_model", "planner_model"):
                            with http.open(f"http://127.0.0.1:{port}/api/settings?key={setting}", timeout=5) as response:
                                if json.load(response) != expected:
                                    raise RuntimeError(f"Model selection did not persist {setting}")
                        report["checks"]["terminal_ollama_model_selection"] = {"models": models, "selected": expected}
                    else:
                        os.write(master, b"\x1b")
                        read_terminal(master, 0.5)
                        report["checks"]["terminal_ollama_model_selection"] = "No installed models; empty picker verified"
                    os.write(master, "qé🦀界".encode())
                    read_terminal(master, 0.5)
                    if tui.poll() is not None:
                        raise RuntimeError("Typing ordinary q/Unicode characters exited the terminal")
                    os.write(master, b"\x11")  # Ctrl+Q
                    read_terminal(master, 0.5)
                    if tui.wait(timeout=5) != 0:
                        raise RuntimeError("Terminal did not exit cleanly with Ctrl+Q")
                    report["checks"]["terminal_start_typing_and_quit"] = "passed"
                finally:
                    if tui is not None:
                        stop(tui)
                    os.close(master)
                    if slave is not None:
                        os.close(slave)
            finally:
                stop(process)
            if process.returncode != 0:
                raise RuntimeError(f"Daemon shutdown failed: {process.returncode}")
            report["checks"]["daemon_graceful_shutdown"] = "passed"

            # Exercise auth and encrypted settings against the release executable.
            auth_env = dict(env, OVERSEER_API_TOKEN="temporary-launch-verification-token")
            process = subprocess.Popen([str(daemon), "--server", "--port", str(port)], cwd=root, env=auth_env,
                                       stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 30
                while True:
                    try:
                        http.open(f"http://127.0.0.1:{port}/healthz", timeout=4).close()
                        break
                    except (urllib.error.URLError, TimeoutError):
                        if process.poll() is not None or time.monotonic() > deadline:
                            raise RuntimeError("Authenticated daemon failed to start")
                        time.sleep(0.2)
                try:
                    http.open(f"http://127.0.0.1:{port}/api/settings?key=llm_provider", timeout=5)
                    raise RuntimeError("API accepted a missing token")
                except urllib.error.HTTPError as error:
                    if error.code != 401: raise
                headers = {"Authorization":"Bearer temporary-launch-verification-token", "Content-Type":"application/json"}
                request = urllib.request.Request(f"http://127.0.0.1:{port}/api/settings", headers=headers,
                    data=json.dumps({"key":"openai_api_key", "value":"temporary-test-key"}).encode())
                http.open(request, timeout=5).close()
                with sqlite3.connect(Path(scratch) / "overseer.db") as connection:
                    stored = connection.execute("SELECT value FROM settings WHERE key='openai_api_key'").fetchone()[0]
                if not stored.startswith("v2:") or "temporary-test-key" in stored:
                    raise RuntimeError("Cloud key was not encrypted in SQLite")
                request = urllib.request.Request(f"http://127.0.0.1:{port}/api/settings?key=openai_api_key", headers=headers)
                with http.open(request, timeout=5) as response:
                    if json.load(response) != "temporary-test-key":
                        raise RuntimeError("Saved cloud key could not be decrypted")
                with socket.create_connection(("127.0.0.1", port), timeout=5) as ws:
                    handshake = (f"GET /api/ws/chat HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
                        "Upgrade: websocket\r\nConnection: Upgrade\r\n"
                        f"Sec-WebSocket-Key: {base64.b64encode(os.urandom(16)).decode()}\r\nSec-WebSocket-Version: 13\r\n\r\n")
                    ws.sendall(handshake.encode())
                    if b" 101 " not in ws.recv(4096).split(b"\r\n", 1)[0]:
                        raise RuntimeError("Authenticated WebSocket upgrade failed")
                    send_ws_json(ws, {"token":"wrong-token"})
                    if receive_ws_json(ws).get("error") != "unauthorized":
                        raise RuntimeError("WebSocket accepted a wrong token")
                with socket.create_connection(("127.0.0.1", port), timeout=5) as ws:
                    ws.sendall(handshake.encode())
                    if b" 101 " not in ws.recv(4096).split(b"\r\n", 1)[0]:
                        raise RuntimeError("Authenticated WebSocket upgrade failed")
                    send_ws_json(ws, {"token":"temporary-launch-verification-token"})
                    send_ws_json(ws, {"conversation_id":"0", "message":"/memory list"})
                    if receive_ws_json(ws).get("type") != "session" or receive_ws_json(ws).get("type") != "done":
                        raise RuntimeError("Authenticated WebSocket chat failed")
                report["checks"]["websocket_token_authentication"] = "passed"
                key = Path(scratch) / "credentials.key"
                if key.stat().st_mode & 0o777 != 0o600:
                    raise RuntimeError("Credential key permissions are too broad")
                subprocess.run([str(daemon), "backup"], env=auth_env, cwd=root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                backups = list((Path(scratch) / "backups").glob("*.key"))
                if not backups or backups[0].read_bytes() != key.read_bytes():
                    raise RuntimeError("Database backup did not preserve the credential key")
                report["checks"]["api_token_encrypted_cloud_keys_and_backup"] = "passed"
            finally:
                stop(process)
            if process.returncode != 0:
                raise RuntimeError("Authenticated daemon did not shut down cleanly")

    destination = root / "dist/macos-build-verification.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    print(f"Verification report: {destination}")


if __name__ == "__main__":
    main()
