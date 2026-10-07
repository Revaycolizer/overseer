"""Verify all SDKs against an isolated local daemon, never the resident database.

Build the daemon and TypeScript SDK first. Run in a Python environment containing
an editable Python SDK install plus its streaming extra. Uses Node and Cargo.
"""
import json,os,secrets,signal,socket,subprocess,sys,tempfile,time
from pathlib import Path
from urllib.request import urlopen
ROOT=Path(__file__).resolve().parents[1]
binary=ROOT/'target/release/overseer'
if not binary.exists():raise SystemExit('Build the daemon first: cargo build --locked --release --bin overseer')
commands=[
    ['node','sdks/typescript/tests/live.mjs'],
    [sys.executable,'sdks/python/tests/live.py'],
    ['cargo','run','--locked','--quiet','-p','overseer-client','--example','sdk-smoke'],
]
# Each SDK gets its own daemon so the shared per-IP burst limiter cannot make
# one SDK's contract checks consume the next SDK's request budget.
for command in commands:
    with tempfile.TemporaryDirectory(prefix='overseer-sdk-verification-') as temp:
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1',0));port=reservation.getsockname()[1]
        env={**os.environ,'OVERSEER_DATA_DIR':temp,'OVERSEER_API_TOKEN':secrets.token_hex(24),'OVERSEER_BASE_URL':f'http://127.0.0.1:{port}'}
        with open(Path(temp)/'daemon.log','w') as log:
            daemon=subprocess.Popen([str(binary),'--server','--port',str(port)],cwd=ROOT,env=env,stdout=log,stderr=log)
            try:
                for _ in range(150):
                    if daemon.poll() is not None:raise RuntimeError('Temporary daemon exited during startup')
                    try:
                        with urlopen(env['OVERSEER_BASE_URL']+'/healthz',timeout=3) as response:
                            if json.load(response)['db_connected']:break
                    except Exception:time.sleep(.1)
                else:raise RuntimeError('Temporary daemon did not start')
                subprocess.run(command,cwd=ROOT,env=env,check=True,timeout=180)
            finally:
                daemon.send_signal(signal.SIGINT)
                try:daemon.wait(timeout=10)
                except subprocess.TimeoutExpired:daemon.kill();daemon.wait()
print('All SDK live checks passed against temporary authenticated daemons.')
