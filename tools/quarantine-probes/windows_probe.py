"""Manual Win32 experiments on temporary files; not a quarantine implementation."""
import ctypes
from ctypes import wintypes
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

if sys.platform != "win32":
    raise SystemExit("This probe must run on Windows")

k = ctypes.WinDLL("kernel32", use_last_error=True)
k.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
k.CreateFileW.restype = wintypes.HANDLE
k.CloseHandle.argtypes = [wintypes.HANDLE]
k.CloseHandle.restype = wintypes.BOOL
k.SetFileInformationByHandle.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
k.SetFileInformationByHandle.restype = wintypes.BOOL


class FileInfo(ctypes.Structure):
    _fields_ = [("attributes", wintypes.DWORD), ("creation", wintypes.FILETIME),
                ("access", wintypes.FILETIME), ("write", wintypes.FILETIME),
                ("volume", wintypes.DWORD), ("size_hi", wintypes.DWORD),
                ("size_lo", wintypes.DWORD), ("links", wintypes.DWORD),
                ("id_hi", wintypes.DWORD), ("id_lo", wintypes.DWORD)]


k.GetFileInformationByHandle.argtypes = [wintypes.HANDLE, ctypes.POINTER(FileInfo)]
k.GetFileInformationByHandle.restype = wintypes.BOOL
INVALID = ctypes.c_void_p(-1).value
GENERIC_READ, DELETE = 0x80000000, 0x00010000
SHARE_READ, SHARE_WRITE, SHARE_DELETE = 1, 2, 4
OPEN_EXISTING, OPEN_REPARSE, BACKUP_SEMANTICS = 3, 0x00200000, 0x02000000
REPARSE_ATTRIBUTE = 0x400


def open_handle(path, access, share, flags=0):
    h = k.CreateFileW(str(path), access, share, None, OPEN_EXISTING, flags, None)
    if h == INVALID:
        raise ctypes.WinError(ctypes.get_last_error())
    return h


def info(handle):
    result = FileInfo()
    if not k.GetFileInformationByHandle(handle, ctypes.byref(result)):
        raise ctypes.WinError(ctypes.get_last_error())
    return result


def rename_relative(handle, root_handle, name):
    class RenameInfo(ctypes.Structure):
        _fields_ = [("replace", ctypes.c_ubyte), ("root", wintypes.HANDLE),
                    ("length", wintypes.DWORD), ("name", ctypes.c_wchar * len(name))]
    arg = RenameInfo(0, root_handle, len(name.encode("utf-16-le")), name)
    return bool(k.SetFileInformationByHandle(handle, 3, ctypes.byref(arg), ctypes.sizeof(arg)))


class WindowsFileProbe(unittest.TestCase):
    def test_share_modes_and_handle_rename_no_replace(self):
        with tempfile.TemporaryDirectory(prefix="ochistka-probe-") as d:
            root = pathlib.Path(d)
            source, target = root / "source", root / "destination"
            other = root / "different"
            source.write_bytes(b"checked")
            target.write_bytes(b"existing")
            other.write_bytes(b"checked")
            directory = open_handle(root, GENERIC_READ, 7, BACKUP_SEMANTICS)
            handle = open_handle(source, GENERIC_READ | DELETE, SHARE_READ, OPEN_REPARSE)
            try:
                before = info(handle)
                self.assertFalse(before.attributes & REPARSE_ATTRIBUTE)
                other_handle = open_handle(other, GENERIC_READ, 7, OPEN_REPARSE)
                try:
                    other_id = info(other_handle)
                    self.assertNotEqual((before.volume, before.id_hi, before.id_lo),
                                        (other_id.volume, other_id.id_hi, other_id.id_lo))
                finally:
                    k.CloseHandle(other_handle)
                code = ("import os,sys; p=sys.argv[1]; "
                        "\ntry:\n f=open(p,'wb'); f.close(); sys.exit(1)\nexcept OSError: pass\n"
                        "try:\n os.rename(p,sys.argv[2]); sys.exit(2)\nexcept OSError: sys.exit(0)")
                result = subprocess.run([sys.executable, "-c", code, str(source), str(root / "other")], check=False)
                self.assertEqual(result.returncode, 0, "second process wrote or renamed the held file")
                self.assertFalse(rename_relative(handle, directory, "destination"))
                self.assertIn(ctypes.get_last_error(), (80, 183))
                self.assertEqual(source.read_bytes(), b"checked")
                self.assertEqual(target.read_bytes(), b"existing")
                after = info(handle)
                self.assertEqual((before.volume, before.id_hi, before.id_lo), (after.volume, after.id_hi, after.id_lo))
                target.unlink()
                self.assertTrue(rename_relative(handle, directory, "destination"), ctypes.get_last_error())
                self.assertFalse(source.exists())
                self.assertEqual(target.read_bytes(), b"checked")
                renamed = info(handle)
                self.assertEqual((before.volume, before.id_hi, before.id_lo), (renamed.volume, renamed.id_hi, renamed.id_lo))
                print("OBSERVED: current share flags rejected competing write/rename and destination was preserved")
            finally:
                k.CloseHandle(handle)
                k.CloseHandle(directory)

    def test_open_reparse_point_without_following_it(self):
        with tempfile.TemporaryDirectory(prefix="ochistka-probe-") as d:
            root = pathlib.Path(d)
            source, link = root / "source", root / "link"
            source.write_bytes(b"checked")
            try:
                link.symlink_to(source)
            except OSError as err:
                self.skipTest(f"runner cannot create a symlink: {err}")
            handle = open_handle(link, 0, 7, OPEN_REPARSE)
            try:
                self.assertTrue(info(handle).attributes & REPARSE_ATTRIBUTE)
                self.assertEqual(source.read_bytes(), b"checked")
            finally:
                k.CloseHandle(handle)


if __name__ == "__main__":
    unittest.main(verbosity=2)
