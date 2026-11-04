"""Run install-app.sh against a temporary HOME, with the macOS-only steps stubbed."""

import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "install-app.sh"

# ditto, codesign and open are macOS tools; pgrep finds no running StatGlass.
STUBS = {
    "ditto": 'cp -R "$1" "$2"\n',
    "codesign": "exit 0\n",
    "pgrep": "exit 1\n",
    "pkill": "exit 0\n",
    "open": 'echo "$@" > "$HOME/opened"\n',
}


class InstallAppLinkTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="statglass install ")
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        self.home = root / "home"
        self.home.mkdir()
        stubs = root / "stubs"
        stubs.mkdir()
        for name, body in STUBS.items():
            path = stubs / name
            path.write_text("#!/bin/sh\n" + body)
            path.chmod(0o755)
        self.app = root / "build/StatGlass.app"
        macos = self.app / "Contents/MacOS"
        macos.mkdir(parents=True)
        cli = macos / "statglass-cli"
        cli.write_text("#!/bin/sh\n")
        cli.chmod(0o755)
        self.bin = self.home / ".local/bin"
        self.link = self.bin / "statglass"
        self.target = str(self.home / "Applications/StatGlass.app/Contents/MacOS/statglass-cli")
        self.env = {
            "HOME": str(self.home),
            "PATH": f"{stubs}:{os.environ['PATH']}",
            "UID": str(os.getuid()),
        }

    def install(self):
        result = subprocess.run(
            ["bash", str(SCRIPT), str(self.app)],
            env=self.env, capture_output=True, text=True, timeout=60,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.home / "opened").exists(), "the app is opened")
        return result

    def test_fresh_install_links_the_bundled_cli(self):
        result = self.install()
        self.assertEqual(os.readlink(self.link), self.target)
        self.assertIn("linked", result.stdout)
        self.assertEqual(sorted(p.name for p in self.bin.iterdir()), ["statglass"])

    def test_repoints_a_link_to_another_copy(self):
        self.bin.mkdir(parents=True)
        os.symlink("/Volumes/Old/StatGlass.app/Contents/MacOS/statglass-cli", self.link)
        self.install()
        self.assertEqual(os.readlink(self.link), self.target)

    def test_leaves_a_foreign_link_alone(self):
        self.bin.mkdir(parents=True)
        os.symlink("/usr/local/bin/other", self.link)
        result = self.install()
        self.assertEqual(os.readlink(self.link), "/usr/local/bin/other")
        self.assertIn("points to /usr/local/bin/other, not StatGlass", result.stderr)

    def test_leaves_a_regular_file_alone(self):
        self.bin.mkdir(parents=True)
        self.link.write_text("mine\n")
        result = self.install()
        self.assertEqual(self.link.read_text(), "mine\n")
        self.assertIn("is not a link", result.stderr)

    def test_leaves_a_link_to_a_directory_alone(self):
        self.bin.mkdir(parents=True)
        elsewhere = self.home / "elsewhere"
        elsewhere.mkdir()
        os.symlink(elsewhere, self.link)
        self.install()
        self.assertEqual(os.readlink(self.link), str(elsewhere))
        self.assertEqual(list(elsewhere.iterdir()), [])

    def test_replaces_our_link_even_when_it_resolves_to_a_directory(self):
        # `ln -sfn`/`mv` would follow this link and drop the new one inside the directory.
        decoy = self.home / "Decoy.app/Contents/MacOS/statglass-cli"
        decoy.mkdir(parents=True)
        self.bin.mkdir(parents=True)
        os.symlink(decoy, self.link)
        self.install()
        self.assertEqual(os.readlink(self.link), self.target)
        self.assertEqual(list(decoy.iterdir()), [])

    def test_a_local_that_is_a_file_does_not_stop_the_install(self):
        (self.home / ".local").write_text("not a directory\n")
        result = self.install()
        self.assertIn("could not create", result.stderr)
        self.assertTrue((self.home / "Applications/StatGlass.app").is_dir())

    def test_a_bin_others_can_write_is_not_linked_into(self):
        self.bin.mkdir(parents=True)
        self.bin.chmod(0o775)
        result = self.install()
        self.assertIn("writable by others", result.stderr)
        self.assertIn("ln -sfn", result.stderr)
        self.assertFalse(os.path.lexists(self.link))
        self.assertTrue((self.home / "Applications/StatGlass.app").is_dir())

    @unittest.skipIf(os.geteuid() == 0, "root can write a read-only directory")
    def test_an_unwritable_bin_does_not_stop_the_install(self):
        self.bin.mkdir(parents=True)
        self.bin.chmod(stat.S_IRUSR | stat.S_IXUSR)
        self.addCleanup(self.bin.chmod, 0o755)
        result = self.install()
        self.assertIn("could not link", result.stderr)
        self.assertFalse(os.path.lexists(self.link))


if __name__ == "__main__":
    unittest.main()
