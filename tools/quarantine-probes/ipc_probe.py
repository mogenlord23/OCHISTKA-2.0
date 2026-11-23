"""Inspect local peer identities; no privileged helper or file operations on behalf of clients."""
import ctypes
import os
import socket
import subprocess
import sys
import tempfile
import uuid


def macos():
    libc = ctypes.CDLL(None, use_errno=True)
    libc.getpeereid.argtypes = [ctypes.c_int, ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint)]
    libc.getpeereid.restype = ctypes.c_int
    with tempfile.TemporaryDirectory(prefix="ochistka-ipc-") as d:
        endpoint = os.path.join(d, "s")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
            server.bind(endpoint)
            server.listen(2)
            seen = []
            for _ in range(2):
                code = "import socket,sys;s=socket.socket(socket.AF_UNIX);s.connect(sys.argv[1]);s.sendall(b'x');s.close()"
                client = subprocess.Popen([sys.executable, "-c", code, endpoint])
                with server.accept()[0] as peer:
                    uid, gid = ctypes.c_uint(), ctypes.c_uint()
                    if libc.getpeereid(peer.fileno(), ctypes.byref(uid), ctypes.byref(gid)) != 0:
                        raise OSError(ctypes.get_errno(), "getpeereid failed")
                    assert peer.recv(1) == b"x"
                    seen.append((uid.value, gid.value))
                assert client.wait() == 0
            assert seen[0] == seen[1]
            print("OBSERVED: getpeereid returns the same UID/GID for both clients; UID/GID alone is not authorization")


def windows():
    from ctypes import wintypes
    k = ctypes.WinDLL("kernel32", use_last_error=True)
    k.CreateNamedPipeW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p]
    k.CreateNamedPipeW.restype = wintypes.HANDLE
    k.ConnectNamedPipe.argtypes = [wintypes.HANDLE, ctypes.c_void_p]
    k.ConnectNamedPipe.restype = wintypes.BOOL
    k.GetNamedPipeClientProcessId.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.ULONG)]
    k.GetNamedPipeClientProcessId.restype = wintypes.BOOL
    k.DisconnectNamedPipe.argtypes = [wintypes.HANDLE]
    k.CloseHandle.argtypes = [wintypes.HANDLE]
    endpoint = r"\\.\pipe\ochistka-probe-" + uuid.uuid4().hex
    seen = []
    for _ in range(2):
        handle = k.CreateNamedPipeW(endpoint, 1, 0, 1, 1024, 1024, 0, None)
        if handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            code = "import sys;f=open(sys.argv[1],'wb',buffering=0);f.write(b'x');sys.stdin.buffer.read(1);f.close()"
            client = subprocess.Popen([sys.executable, "-c", code, endpoint], stdin=subprocess.PIPE)
            connected = k.ConnectNamedPipe(handle, None)
            if not connected and ctypes.get_last_error() != 535:  # ERROR_PIPE_CONNECTED
                raise ctypes.WinError(ctypes.get_last_error())
            pid = wintypes.ULONG()
            if not k.GetNamedPipeClientProcessId(handle, ctypes.byref(pid)):
                raise ctypes.WinError(ctypes.get_last_error())
            assert pid.value == client.pid
            seen.append(pid.value)
            client.stdin.write(b"x")
            client.stdin.close()
            assert client.wait(timeout=10) == 0
            k.DisconnectNamedPipe(handle)
        finally:
            k.CloseHandle(handle)
    assert seen[0] != seen[1]
    print("OBSERVED: distinct client PIDs; PID alone is not authenticated application identity or authorization")


if __name__ == "__main__":
    if sys.platform == "darwin":
        macos()
    elif sys.platform == "win32":
        windows()
    else:
        raise SystemExit("This probe must run on Windows or macOS")
