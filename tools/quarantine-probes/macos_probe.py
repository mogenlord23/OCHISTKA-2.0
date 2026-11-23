"""Manual macOS experiments on temporary files; not a quarantine implementation."""
import ctypes
import errno
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

if sys.platform != "darwin":
    raise SystemExit("This probe must run on macOS")

libc = ctypes.CDLL(None, use_errno=True)
rename_exclusive = libc.renameatx_np
rename_exclusive.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
rename_exclusive.restype = ctypes.c_int
RENAME_EXCL = 0x00000004


class MacFileProbe(unittest.TestCase):
    def test_nofollow_and_identity(self):
        with tempfile.TemporaryDirectory(prefix="ochistka-probe-") as d:
            root = pathlib.Path(d)
            path, link = root / "file", root / "link"
            path.write_bytes(b"checked")
            link.symlink_to(path)
            with self.assertRaises(OSError) as err:
                os.open(link, os.O_RDONLY | os.O_NOFOLLOW)
            self.assertEqual(err.exception.errno, errno.ELOOP)
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                opened, named = os.fstat(fd), path.stat()
                self.assertEqual((opened.st_dev, opened.st_ino), (named.st_dev, named.st_ino))
            finally:
                os.close(fd)

    def test_exclusive_rename_conflict_and_replacement_race(self):
        with tempfile.TemporaryDirectory(prefix="ochistka-probe-") as d:
            root = pathlib.Path(d)
            source, old, dest = (root / n for n in ("source", "original", "destination"))
            source.write_bytes(b"checked")
            dest.write_bytes(b"existing")
            directory = os.open(root, os.O_RDONLY)
            fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                result = rename_exclusive(directory, b"source", directory, b"destination", RENAME_EXCL)
                self.assertEqual(result, -1)
                self.assertEqual(ctypes.get_errno(), errno.EEXIST)
                self.assertEqual(dest.read_bytes(), b"existing")
                dest.unlink()
                # Child completes replacement before the parent continues: no timing assumptions.
                code = "import os,sys;os.rename(sys.argv[1],sys.argv[2]);open(sys.argv[1],'wb').write(b'replacement')"
                subprocess.run([sys.executable, "-c", code, str(source), str(old)], check=True)
                self.assertNotEqual(os.fstat(fd).st_ino, source.stat().st_ino)
                self.assertEqual(rename_exclusive(directory, b"source", directory, b"destination", RENAME_EXCL), 0)
                self.assertEqual(dest.read_bytes(), b"replacement")
                self.assertEqual(old.read_bytes(), b"checked")
                print("OBSERVED: no-replace rename by name moved replacement after source swap")
            finally:
                os.close(fd)
                os.close(directory)

    def test_open_descriptor_does_not_block_other_process_writing(self):
        with tempfile.TemporaryDirectory(prefix="ochistka-probe-") as d:
            path = pathlib.Path(d) / "source"
            path.write_bytes(b"checked")
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
            try:
                code = "import sys;open(sys.argv[1],'wb').write(b'changed')"
                subprocess.run([sys.executable, "-c", code, str(path)], check=True)
                self.assertEqual(os.read(fd, 7), b"changed")
                print("OBSERVED: another process modified the file while the read descriptor remained open")
            finally:
                os.close(fd)


if __name__ == "__main__":
    unittest.main(verbosity=2)
