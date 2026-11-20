"""Verify native dialogue through a loopback gateway executable and a temporary SQLite store."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import URLError
from urllib.parse import urlencode
from urllib.request import ProxyHandler, Request, build_opener
from uuid import uuid4


MODEL = "native-disconnect-probe-model"
INPUT = "Native reconnect probe. Respond without tools."
ANSWER = "The original response was committed."
DIGEST = hashlib.sha256(INPUT.encode()).hexdigest()
OPENER = build_opener(ProxyHandler({}))


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def call(port, token, path, body=None):
    headers = {"Authorization": "Bearer " + token}
    data = None
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode()
    request = Request(f"http://127.0.0.1:{port}{path}", data=data, headers=headers)
    with OPENER.open(request, timeout=2) as response:
        return json.load(response)


class Provider(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def reply(self, status, value):
        raw = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def do_POST(self):
        provider = self.server
        if self.path != "/v1/chat/completions" or self.headers.get("Authorization") != "Bearer " + provider.key:
            self.reply(401, {})
            return
        size = int(self.headers["Content-Length"])
        require(0 < size <= 1024 * 1024, "unexpected provider fixture input size")
        request = json.loads(self.rfile.read(size))
        require(request["model"] == MODEL and request["messages"][-1]["content"] == INPUT,
                "provider received the wrong model or user input")
        with provider.lock:
            provider.calls += 1
            require(provider.calls == 1, "provider called again for a single native round")
        provider.entered.set()
        require(provider.release.wait(15), "fixture release timed out")
        try:
            self.reply(200, {
                "id": "native-disconnect-result", "object": "chat.completion", "created": 1,
                "model": MODEL, "choices": [{"index": 0, "finish_reason": "stop",
                    "message": {"role": "assistant", "content": ANSWER}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 8, "total_tokens": 18},
            })
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            # The negative binary cancels the provider's own HTTP request.
            return


def start_gateway(executable, root, port, token, provider):
    literal = {"systemroot", "windir", "path", "comspec", "pathext", "systemdrive",
               "programdata", "allusersprofile", "os"}
    env = {key: value for key, value in os.environ.items() if key.lower() in literal}
    for key in ("USERPROFILE", "LOCALAPPDATA", "APPDATA", "TEMP", "TMP"):
        isolated = root / key.lower()
        isolated.mkdir(exist_ok=True)
        env[key] = str(isolated)
    env.update(APEIRETH_GATEWAY_TOKEN=token, APEIRETH_DATA_DIR=str(root / "data"),
               APEIRETH_SESSION_DB=str(root / "sessions.sqlite3"),
               APEIRETH_COGNITIVE_DB=str(root / "cognitive.sqlite3"),
               APEIRETH_MODEL=MODEL, APEIRETH_OPENAI_MODELS=MODEL,
               OPENAI_API_KEY=provider.key,
               APEIRETH_OPENAI_URL=f"http://127.0.0.1:{provider.server_port}/v1",
               APEIRETH_MAX_TURN_ROUNDS="2", APEIRETH_DISABLE_LOCAL_READ_TOOLS="1")
    args = [str(executable), "gateway", "serve", "--bind", "127.0.0.1", "--port", str(port),
            "--pid-file", str(root / "gateway.pid")]
    creationflags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with (root / "gateway.log").open("ab") as log:
        process = subprocess.Popen(args, env=env, cwd=root, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT, creationflags=creationflags)
    try:
        for _ in range(150):
            require(process.poll() is None, "isolated gateway exited during startup")
            try:
                health = call(port, token, "/health")
                snapshot = call(port, token, "/v1/runtime/snapshot")
                require(health["execution_owner"] == "apeireth-runtime::canonical" and snapshot,
                        "unexpected runtime")
                return process
            except (URLError, TimeoutError):
                time.sleep(0.1)
        raise RuntimeError("isolated gateway did not become ready")
    except BaseException:
        stop_gateway(process, port, token)
        raise


def stop_gateway(process, port, token):
    if process.poll() is not None:
        return
    try:
        call(port, token, "/v1/admin/shutdown", {})
        process.wait(10)
    except (URLError, TimeoutError, subprocess.TimeoutExpired):
        process.terminate()
        process.wait(10)


def verify_round(executable, reset):
    with tempfile.TemporaryDirectory(prefix="apeireth-native-disconnect-") as scratch, \
            ThreadingHTTPServer(("127.0.0.1", 0), Provider) as provider:
        root = Path(scratch).resolve()
        provider.key = "fixture-" + uuid4().hex
        provider.entered = threading.Event()
        provider.release = threading.Event()
        provider.lock = threading.Lock()
        provider.calls = 0
        thread = threading.Thread(target=provider.serve_forever, daemon=True)
        thread.start()
        with socket.socket() as slot:
            slot.bind(("127.0.0.1", 0))
            port = slot.getsockname()[1]
        token = "fixture-gateway-" + uuid4().hex
        process = None
        try:
            process = start_gateway(executable, root, port, token, provider)
            session = str(uuid4())
            path = f"/v1/sessions/{session}"
            body = json.dumps({"session": session, "input": INPUT, "model": MODEL}).encode()
            header = (f"POST /v1/chat HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
                      f"Authorization: Bearer {token}\r\nContent-Type: application/json\r\n"
                      f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n").encode()
            with socket.create_connection(("127.0.0.1", port), timeout=2) as waiter:
                waiter.sendall(header + body)
                require(provider.entered.wait(10), "original turn never reached the provider")
                if reset:
                    waiter.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER,
                                     struct.pack("HH" if os.name == "nt" else "ii", 1, 0))
            pending = call(port, token, path + "/turns")["turns"]
            require(len(pending) == 1 and pending[0]["state"] == "unconfirmed" and pending[0]["text"] is None,
                    "history did not identify one waiting original turn")
            identity = {"request": pending[0]["request"], "trace_id": pending[0]["trace_id"],
                        "input_sha256": DIGEST}
            read = path + "/turn-result?" + urlencode(identity)
            provider.release.set()
            for _ in range(70):
                completed = call(port, token, read)
                if completed["state"] == "completed":
                    break
                time.sleep(0.1)
            else:
                raise RuntimeError(f"original turn remained {completed['state']} after disconnect")
            require(completed["text"] == ANSWER and completed["session"] == session
                    and all(completed[key] == value for key, value in identity.items()),
                    "completed result does not match the original identity")
            stop_gateway(process, port, token)
            process = start_gateway(executable, root, port, token, provider)
            require(call(port, token, read) == completed, "completed result changed after restart")
            committed = call(port, token, path + "/turns")["turns"]
            require(len(committed) == 1 and committed[0]["state"] == "completed"
                    and committed[0]["text"] == ANSWER and provider.calls == 1,
                    "restart read must preserve one turn and one provider call")
            print(f"PASS {'reset' if reset else 'close'}: original turn completed once and survived restart")
        finally:
            provider.release.set()
            if process is not None:
                stop_gateway(process, port, token)
            provider.shutdown()
            thread.join(5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--sha256", required=True, help="expected digest of the isolated executable")
    parser.add_argument("--mode", choices=("both", "close", "reset"), default="both")
    args = parser.parse_args()
    binary = args.executable.resolve(strict=True)
    with binary.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == args.sha256.lower(),
                "gateway executable digest mismatch")
    for mode in ("close", "reset") if args.mode == "both" else (args.mode,):
        verify_round(binary, reset=mode == "reset")
