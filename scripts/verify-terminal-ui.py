#!/usr/bin/env python3
"""POSIX native terminal acceptance smoke with isolated data and installation."""
import argparse, re, fcntl, os, pathlib, pty, select, struct, subprocess, tarfile, tempfile, termios, time
root=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary',type=pathlib.Path,help='Verify a rebuilt binary directly instead of the distribution archive')
args=parser.parse_args()
with tempfile.TemporaryDirectory() as tmp:
    task=pathlib.Path(tmp)
    env=dict(os.environ,OVERSEER_INSTALL_DIR=str(task/'bin'),OVERSEER_DATA_DIR=str(task/'data'),TERM='xterm-256color')
    if args.binary:
        binary=args.binary.resolve(strict=True)
    else:
        archive=next((root/'dist/terminal').glob('*.tar.gz'))
        with tarfile.open(archive) as t: t.extractall(task,filter='data')
        stage=next(task.iterdir())
        subprocess.run(['sh',str(stage/'install-terminal.sh')],env=env,check=True,stdout=subprocess.DEVNULL)
        binary=task/'bin/overseer-tui'
    master,slave=pty.openpty()
    fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,0,0))
    proc=subprocess.Popen([str(binary)],cwd=task,env=env,stdin=slave,stdout=slave,stderr=slave)
    os.close(slave)
    def read(seconds):
        output=b''; end=time.monotonic()+seconds
        while time.monotonic()<end:
            if select.select([master],[],[],.1)[0]:
                try: output+=os.read(master,65536)
                except OSError: break
        return output
    try:
        output=read(4)
        assert proc.poll() is None and b'Overseer' in output, output[-2000:]
        os.write(master,b'helloterminal')
        output=read(1)
        assert b'helloterminal' in re.sub(rb'\s+', b'', re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', output)), output[-2000:]
        os.write(master,b'\x11')
        read(1)
        assert proc.wait(timeout=5)==0
        assert (task/'data/overseer.db').exists()
        print('Standalone UI: launch from outside repository, type, Ctrl+Q, embedded migrations passed.')
    finally:
        if proc.poll() is None: proc.kill(); proc.wait()
        os.close(master)
