#!/usr/bin/env python3
"""Mac native acceptance: bundled backend, shared child, auth and crash cleanup."""
import json, os, pathlib, re, subprocess, tempfile, time, urllib.request, urllib.error
root=pathlib.Path(__file__).resolve().parents[1]
bundle=root/'desktop-ui/src-tauri/target/release/bundle/macos/Overseer.app'
binary=bundle/'Contents/MacOS/desktop-ui'
assert (bundle/'Contents/Resources/backend/overseer').is_file()
subprocess.run(['codesign','--verify','--deep','--strict',str(bundle)],check=True)
with tempfile.TemporaryDirectory(prefix='overseer-native-desktop-') as tmp:
    env=dict(os.environ,OVERSEER_DATA_DIR=str(pathlib.Path(tmp)/'data'))
    for cycle in range(2):
        with open(pathlib.Path(tmp)/'app.log','ab') as output:
            app=subprocess.Popen([str(binary)],cwd=tmp,env=env,stdout=output,stderr=output)
        child=None
        try:
            for _ in range(150):
                assert app.poll() is None, 'Desktop exited before connecting'
                children=subprocess.run(['pgrep','-P',str(app.pid)],capture_output=True,text=True).stdout.split()
                candidates=[]
                for pid in children:
                    command=subprocess.run(['ps','-p',pid,'-o','command='],capture_output=True,text=True).stdout
                    if '/backend/overseer' in command: candidates.append((int(pid),command))
                if candidates:
                    assert len(candidates)==1, 'Both windows must share one backend'
                    child,command=candidates[0];port=int(re.search(r'--port (\d+)',command)[1])
                    assert port!=8080
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{port}/healthz',timeout=1) as response:
                            health=json.load(response)
                        assert health['db_connected']
                        break
                    except (urllib.error.URLError,TimeoutError): pass
                time.sleep(.1)
            else: raise RuntimeError('Bundled backend did not become healthy')
            try:
                urllib.request.urlopen(f'http://127.0.0.1:{port}/api/messages?conversation_id=1',timeout=2)
                raise AssertionError('Managed backend must require authentication')
            except urllib.error.HTTPError as error: assert error.code==401
            app.terminate();app.wait(timeout=5)
            for _ in range(60):
                running=subprocess.run(['ps','-p',str(child),'-o','stat='],capture_output=True,text=True).stdout.strip()
                if not running or running.startswith('Z'): break
                time.sleep(.1)
            else: raise RuntimeError('Owned backend survived desktop termination')
            assert (pathlib.Path(tmp)/'data/overseer.db').is_file()
        finally:
            if app.poll() is None: app.kill();app.wait()
            if child:
                running=subprocess.run(['ps','-p',str(child),'-o','stat='],capture_output=True,text=True).stdout.strip()
                if running and not running.startswith('Z'):
                    os.kill(child,9)
print('PASS: bundle signature/resources, authenticated backend, one child across windows, isolated port, crash cleanup and relaunch.')
