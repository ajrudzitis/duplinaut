#!/usr/bin/env python3
"""Local controlled PTY evidence, no source mutation or credentials."""
import argparse, fcntl, json, os, pathlib, pty, re, select, struct, subprocess, tempfile, termios, time
parser=argparse.ArgumentParser();parser.add_argument('binary');parser.add_argument('fixtures');args=parser.parse_args()
master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',32,120,0,0));before=termios.tcgetattr(slave)
with tempfile.TemporaryDirectory(prefix='duplicity-tui-pty-') as directory:
    parent=os.fsencode(directory);name=b'dest-\xff';destination=os.path.join(parent,name)
    process=subprocess.Popen([os.fsencode(args.binary),b'local',os.fsencode(args.fixtures),b'--restore-parent',parent,b'--restore-name',name],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'TERM':'xterm-256color'})
    transcript=bytearray()
    def read_for(seconds):
        deadline=time.monotonic()+seconds
        while time.monotonic()<deadline:
            if select.select([master],[],[],min(.05,max(0,deadline-time.monotonic())))[0]:
                try: transcript.extend(os.read(master,65536))
                except OSError: break
    def wait_for(marker):
        deadline=time.monotonic()+10
        while time.monotonic()<deadline:
            read_for(.05)
            cleaned=re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]',b'',bytes(transcript))
            if marker in cleaned: return
            if process.poll() is not None: raise AssertionError(('child exited',process.returncode,cleaned[-4000:]))
        raise AssertionError(('marker absent',marker,cleaned[-4000:]))
    read_for(1);os.write(master,b'a\r');read_for(.2);os.write(master,b'\r');read_for(1)
    os.write(master,b'p');read_for(.2);assert not os.path.lexists(destination)
    os.write(master,b'rrestore\r');read_for(1);assert not os.path.lexists(destination), 'preparation/pending input must not confirm'
    assert b'dest-\\xff' in transcript, 'escaped raw destination preview must render'
    os.write(master,b'\r');read_for(.2);assert not os.path.lexists(destination),'empty acknowledgment Enter cannot confirm'
    os.write(master,b'restore\r');read_for(1);assert os.path.isdir(destination)
    os.write(master,b'q');process.wait(timeout=10);read_for(.05);after=termios.tcgetattr(slave);assert process.returncode==0;assert before==after,'terminal modes restored'
    print(json.dumps({'exit_code':process.returncode,'destination_bytes_hex':name.hex(),'read_only_preview_no_write':True,'pending_input_no_write':True,'empty_enter_no_write':True,'observed_acknowledgment_publication':True,'terminal_modes_restored':before==after,'transcript_bytes':len(transcript),'limitations':['PTY smoke only, not comprehensive terminal IO fault proof','frontend worker native tests cover three-date source oracles','kernel IO can block without fixed cancellation latency']},indent=2))
os.close(master);os.close(slave)
