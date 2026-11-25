#!/usr/bin/env python3
"""Run the authenticated Builder ingress and compiled Rust bridge as one service."""
from __future__ import annotations

import argparse
import base64
import os
from pathlib import Path
import signal
import socket
import stat
import subprocess
import sys
import threading
import time

from scripts.builder_cloud_ingress import (GatewayConfig, Ingress, load_json,
    private_bridge, private_regular_file, read_frame, write_frame)


class StartupError(RuntimeError):
    """A required service failed before readiness."""


def private_parent(path: Path) -> None:
    parent = path.parent
    if not path.is_absolute() or parent.resolve(strict=True) != parent:
        raise ValueError('private path must have a canonical absolute parent')
    meta = parent.stat()
    if not stat.S_ISDIR(meta.st_mode) or meta.st_uid != os.geteuid() or meta.st_mode & 0o077:
        raise ValueError('private directory must be owned by the service UID')


def private_config(path: Path) -> None:
    private_regular_file(path)


def read_bridge_config(path: Path, origin: str) -> Path:
    private_config(path)
    config = load_json(path.read_bytes())
    if not isinstance(config, dict) or config.get('origin') != origin or not isinstance(config.get('socket'), str):
        raise ValueError('bridge configuration origin or socket mismatch')
    sock = Path(config['socket'])
    private_parent(sock)
    return sock


class BridgeProcess:
    """Own exactly one child. No shell, automatic restart or stale socket deletion."""
    def __init__(self, executable: Path, config: Path, socket_path: Path, *,
                 startup_timeout: float = 20, shutdown_timeout: float = 15,
                 cancel: threading.Event | None = None):
        if not (0 < startup_timeout <= 120 and 0 < shutdown_timeout <= 120):
            raise ValueError('invalid lifecycle timeout')
        self.executable, self.config, self.socket_path = executable, config, socket_path
        self.startup_timeout, self.shutdown_timeout = startup_timeout, shutdown_timeout
        self.cancel = cancel if cancel is not None else threading.Event()
        self.process: subprocess.Popen | None = None

    def start(self) -> None:
        if self.cancel.is_set():
            raise StartupError('bridge startup cancelled')
        if self.process is not None:
            raise StartupError('bridge process already started')
        private_config(self.config)
        private_parent(self.socket_path)
        if os.path.lexists(self.socket_path):
            raise StartupError('bridge socket already exists; no automatic deletion')
        exe = self.executable
        if not exe.is_absolute() or exe.resolve(strict=True) != exe:
            raise StartupError('bridge executable must be a canonical absolute path')
        meta = exe.stat()
        if (not stat.S_ISREG(meta.st_mode) or meta.st_uid not in (0,os.geteuid())
                or meta.st_mode & 0o022 or not os.access(exe, os.X_OK)):
            raise StartupError('untrusted or non-executable bridge')
        self.process = subprocess.Popen([str(exe),str(self.config)], stdin=subprocess.DEVNULL,
                                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                        close_fds=True)
        deadline = time.monotonic() + self.startup_timeout
        try:
            while time.monotonic() < deadline:
                if self.cancel.is_set():
                    raise StartupError('bridge startup cancelled')
                if self.process.poll() is not None:
                    raise StartupError('bridge process exited before readiness')
                try:
                    private_bridge(self.socket_path)
                    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as peer:
                        budget = min(.25, max(.001, deadline-time.monotonic()))
                        peer.settimeout(budget);peer.connect(str(self.socket_path))
                        # The actual bridge rejects a structurally empty envelope before auth/storage.
                        write_frame(peer,{},budget)
                        result=read_frame(peer,budget)
                    if (result == {'status':400,'body_b64':base64.b64encode(b'{"error":"http_400"}').decode('ascii')}
                            and self.process.poll() is None):
                        return
                except (OSError,ValueError,TypeError,RecursionError):
                    pass
                self.cancel.wait(min(.05,max(0,deadline-time.monotonic())))
            raise StartupError('bridge protocol readiness timed out')
        except BaseException:
            self.stop()
            raise

    def stop(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=self.shutdown_timeout)
            except subprocess.TimeoutExpired:
                self.process.kill();self.process.wait()
        else:
            self.process.wait()
        # The child owns its socket; an unclean exit requires operator review.

    def __enter__(self):
        self.start();return self

    def __exit__(self, *_):
        self.stop()


def serve(server: Ingress, bridge: BridgeProcess, stop: threading.Event) -> int:
    """Monitor both components. Failure of either stops the other."""
    serving = threading.Thread(target=server.serve_forever, kwargs={'poll_interval':.1},name='builder-https')
    serving.start()
    try:
        assert bridge.process is not None
        while not stop.wait(.1):
            if bridge.process.poll() is not None or not serving.is_alive():
                return 1
        return 0
    finally:
        if serving.is_alive():
            server.shutdown()
        serving.join()


def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('bridge-bin','bridge-config','cert','key','ca','registry','assets'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--origin',required=True)
    parser.add_argument('--bind',default='127.0.0.1')
    parser.add_argument('--port',type=int,default=8443)
    args=parser.parse_args()
    stop=threading.Event()
    def on_signal(*_):stop.set()
    for sig in (signal.SIGINT,signal.SIGTERM):signal.signal(sig,on_signal)
    try:
        socket_path=read_bridge_config(args.bridge_config,args.origin)
        private_config(args.registry);private_config(args.key)
        identities=load_json(args.registry.read_bytes())
        with BridgeProcess(args.bridge_bin,args.bridge_config,socket_path,cancel=stop) as bridge:
            if stop.is_set():return 0
            config=GatewayConfig(args.origin,args.cert,args.key,args.ca,identities,socket_path,args.assets,
                                 bridge_pid=bridge.process.pid)
            with Ingress((args.bind,args.port),config) as server:
                if bridge.process.poll() is not None:raise StartupError('bridge exited during HTTPS startup')
                print(f'OCAF Builder HTTPS listener ready (protocol probe only) port={server.server_address[1]}',flush=True)
                return serve(server,bridge,stop)
    except StartupError:
        if stop.is_set():
            return 0
        print('OCAF Builder startup failed; bridge did not become ready',file=sys.stderr)
        return 1
    except (ValueError,OSError,RuntimeError,RecursionError):
        print('OCAF Builder startup/runtime failed; inspect private operator configuration and process status',file=sys.stderr)
        return 1


if __name__=='__main__':
    raise SystemExit(main())
