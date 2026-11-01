#!/usr/bin/env python3
"""Exercise the rename command only in disposable Git repositories."""
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('rename-statglass.py')


class RenameTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.invalid')
        self.git('config', 'user.name', 'Test')
        (self.root / 'scripts').mkdir()
        shutil.copy2(SCRIPT, self.root / 'scripts/rename-statglass.py')

    def git(self, *args):
        return subprocess.run(['git', *args], cwd=self.root, check=True, capture_output=True)

    def put(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content if isinstance(content, bytes) else content.encode())
        return target

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture')

    def run_rename(self, *args, code=0):
        result = subprocess.run(['python3', 'scripts/rename-statglass.py', *args], cwd=self.root, capture_output=True, text=True)
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        return result

    def test_dry_run_apply_check_staging_and_idempotence(self):
        original = self.put('kelvo/nested/Kelvo-tool', 'kelvo Kelvo KELVO prekelvopost\n')
        original.chmod(0o755)
        self.commit()
        self.run_rename()
        self.assertTrue(original.exists())
        self.assertEqual(original.read_text(), 'kelvo Kelvo KELVO prekelvopost\n')
        self.run_rename('--check', code=1)
        self.run_rename('--apply')
        renamed = self.root / 'statglass/nested/StatGlass-tool'
        self.assertEqual(renamed.read_text(), 'statglass StatGlass STATGLASS prestatglasspost\n')
        self.assertEqual(stat.S_IMODE(renamed.stat().st_mode), 0o755)
        self.run_rename('--check')
        self.run_rename('--apply')
        self.git('add', '-A')
        self.run_rename('--check')
        self.run_rename('--apply')
        self.assertEqual(renamed.read_text(), 'statglass StatGlass STATGLASS prestatglasspost\n')

    def test_collision_refuses_every_edit(self):
        original = self.put('kelvo.txt', 'kelvo')
        other = self.put('other.txt', 'Kelvo')
        self.commit()
        destination = self.put('statglass.txt', 'keep')
        self.run_rename('--apply', code=2)
        self.assertEqual(original.read_text(), 'kelvo')
        self.assertEqual(other.read_text(), 'Kelvo')
        self.assertEqual(destination.read_text(), 'keep')

    def test_tracked_destination_collision_refuses_every_edit(self):
        original = self.put('kelvo.txt', 'kelvo')
        destination = self.put('statglass.txt', 'keep')
        self.commit()
        self.run_rename('--apply', code=2)
        self.assertEqual(original.read_text(), 'kelvo')
        self.assertEqual(destination.read_text(), 'keep')

    def test_destination_parent_file_refuses_before_any_changes(self):
        original = self.put('kelvo/file.txt', 'kelvo')
        other = self.put('other.txt', 'Kelvo')
        self.commit()
        blocker = self.put('statglass', 'keep')
        result = self.run_rename('--apply', code=2)
        self.assertIn('statglass', result.stderr)
        self.assertEqual(original.read_text(), 'kelvo')
        self.assertEqual(other.read_text(), 'Kelvo')
        self.assertEqual(blocker.read_text(), 'keep')
        self.assertFalse((self.root / 'statglass/file.txt').exists())

    def test_planned_file_and_directory_destinations_refuse_before_changes(self):
        original = self.put('kelvo', 'kelvo')
        other = self.put('statglass/file.txt', 'Kelvo')
        self.commit()
        result = self.run_rename('--apply', code=2)
        self.assertIn('statglass', result.stderr)
        self.assertEqual(original.read_text(), 'kelvo')
        self.assertEqual(other.read_text(), 'Kelvo')
        self.assertTrue((self.root / 'statglass').is_dir())

    def test_source_parent_symlink_cannot_read_or_write_outside_repository(self):
        self.put('kelvo/file.txt', 'kelvo')
        self.commit()
        with tempfile.TemporaryDirectory() as outside:
            external = Path(outside) / 'file.txt'
            external.write_text('kelvo')
            shutil.rmtree(self.root / 'kelvo')
            (self.root / 'kelvo').symlink_to(outside, target_is_directory=True)
            self.run_rename('--apply', code=2)
            self.assertEqual(external.read_text(), 'kelvo')
            self.assertFalse((self.root / 'statglass').exists())

    def test_protected_fixture_remains_unchanged_after_staging_rename(self):
        old = self.put('crates/kelvo-update/tests/fixtures/release.json', 'kelvo')
        self.commit()
        self.run_rename('--apply')
        moved = self.root / 'crates/statglass-update/tests/fixtures/release.json'
        new = moved.with_name('statglass-release.json')
        self.assertFalse(old.exists())
        self.assertEqual(moved.read_text(), 'kelvo')
        moved.rename(new)
        self.run_rename('--check')
        new.write_text('statglass')  # The new catalog is re-signed separately.
        self.git('add', '-A')
        self.run_rename('--check')
        self.run_rename('--apply')
        self.assertEqual(new.read_text(), 'statglass')

    def test_destination_parent_symlink_cannot_write_outside_repository(self):
        original = self.put('kelvo/file.txt', 'kelvo')
        self.commit()
        with tempfile.TemporaryDirectory() as outside:
            (self.root / 'statglass').symlink_to(outside, target_is_directory=True)
            self.run_rename('--apply', code=2)
            self.assertEqual(original.read_text(), 'kelvo')
            self.assertEqual(list(Path(outside).iterdir()), [])

    def test_dirty_affected_file_refused_but_unrelated_dirty_file_allowed(self):
        affected = self.put('a.txt', 'kelvo')
        unrelated = self.put('b.txt', 'original')
        self.commit()
        affected.write_text('kelvo dirty')
        self.run_rename('--apply', code=2)
        self.assertEqual(affected.read_text(), 'kelvo dirty')
        affected.write_text('kelvo')
        unrelated.write_text('dirty')
        self.run_rename('--apply')
        self.assertEqual(affected.read_text(), 'statglass')
        self.assertEqual(unrelated.read_text(), 'dirty')

    def test_binary_generated_ignored_and_untracked_excluded(self):
        binary = self.put('binary.dat', b'kelvo\x00Kelvo')
        generated = self.put('src/core/generated/bindings.ts', 'kelvo')
        self.put('.gitignore', 'ignored/\n')
        self.commit()
        ignored = self.put('ignored/kelvo.txt', 'kelvo')
        untracked = self.put('kelvo-untracked.txt', 'kelvo')
        self.run_rename('--apply')
        self.assertEqual(binary.read_bytes(), b'kelvo\x00Kelvo')
        self.assertEqual(generated.read_text(), 'kelvo')
        self.assertEqual(ignored.read_text(), 'kelvo')
        self.assertEqual(untracked.read_text(), 'kelvo')

    def test_symlink_is_never_followed_and_rename_requires_manual_action(self):
        outside = self.root.parent / (self.root.name + '-outside')
        outside.write_text('kelvo')
        self.addCleanup(outside.unlink)
        (self.root / 'link').symlink_to(outside)
        self.commit()
        self.run_rename('--apply')
        self.assertEqual(outside.read_text(), 'kelvo')
        (self.root / 'kelvo-link').symlink_to(outside)
        self.git('add', 'kelvo-link')
        self.git('commit', '-qm', 'symlink')
        self.run_rename('--apply', code=2)
        self.assertTrue((self.root / 'kelvo-link').is_symlink())
        self.assertEqual(outside.read_text(), 'kelvo')

    def test_rename_test_file_is_excluded(self):
        fixture = self.put('scripts/test-rename-statglass.py', 'kelvo')
        self.commit()
        self.run_rename('--check')
        self.run_rename('--apply')
        self.assertEqual(fixture.read_text(), 'kelvo')

    def test_preserves_documented_literal_exceptions(self):
        source = self.put('references.txt',
                          'https://raw.githubusercontent.com/steamfield/statglass/assets/media/a.webp\n'
                          '~/.config/kelvo/release/updater.key\n'
                          '$HOME/.config/kelvo/release/updater.key\nkelvo\n')
        self.commit()
        self.run_rename('--apply')
        self.assertEqual(source.read_text(),
                         'https://raw.githubusercontent.com/steamfield/statglass/assets/media/a.webp\n'
                         '~/.config/kelvo/release/updater.key\n'
                         '$HOME/.config/kelvo/release/updater.key\nstatglass\n')

    def test_catalog_rejection_url_is_preserved_only_in_its_test(self):
        url = 'https://github.com/steamfield/kelvo/releases/download/v9.0.0/a'
        test = self.put('crates/kelvo-update/src/tests.rs', url)
        ordinary = self.put('other.txt', url)
        self.commit()
        self.run_rename('--apply')
        renamed = self.root / 'crates/statglass-update/src/tests.rs'
        self.assertFalse(test.exists())
        self.assertEqual(renamed.read_text(), url)
        self.assertEqual(ordinary.read_text(), url.replace('/kelvo/', '/statglass/'))

    def test_unexpected_path_casing_refuses_all_changes(self):
        source = self.put('a.txt', 'kelvo')
        self.put('KeLvO.txt', 'untouched')
        self.commit()
        self.run_rename('--apply', code=2)
        self.assertEqual(source.read_text(), 'kelvo')

    def test_unexpected_casing_refuses_all_changes(self):
        source = self.put('a.txt', 'kelvo')
        self.put('b.txt', 'KeLvO')
        self.commit()
        self.run_rename('--apply', code=2)
        self.assertEqual(source.read_text(), 'kelvo')


if __name__ == '__main__':
    unittest.main()
