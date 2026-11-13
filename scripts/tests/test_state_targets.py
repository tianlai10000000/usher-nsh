"""Check that platform verification requires artifacts, not a successful empty build."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('check_state_targets', Path(__file__).resolve().parents[1] / 'check_state_targets.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TargetEvidenceTests(unittest.TestCase):
    def test_complete_matrix_accepts_fresh_or_cached_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            messages = [{'reason': 'build-finished', 'success': True}]
            manifest = Path(temporary) / 'Cargo.toml'
            manifest.write_text('[package]\nname = "library"\nversion = "1.0.0"\n')
            for target in ['target-one', 'target-two']:
                path = Path(temporary) / target / 'library.rmeta'
                path.parent.mkdir()
                path.touch()
                messages.append({'reason': 'compiler-artifact', 'package_id': 'path+file:///library#1.0.0', 'manifest_path': str(manifest), 'target': {'kind': ['lib']}, 'filenames': [str(path)], 'fresh': True})
            self.assertEqual(module.verified_artifacts(messages, ['library'], ['target-one', 'target-two']), {('library', 'target-one'), ('library', 'target-two')})
            with self.assertRaises(ValueError):
                module.verified_artifacts(messages[:-1], ['library'], ['target-one', 'target-two'])
            path.unlink()
            with self.assertRaises(ValueError):
                module.verified_artifacts(messages, ['library'], ['target-one', 'target-two'])

    def test_empty_missing_or_failed_build_cannot_pass(self):
        for messages in [[], [{'reason': 'build-finished', 'success': True}], [{'reason': 'build-finished', 'success': False}]]:
            with self.assertRaises(ValueError):
                module.verified_artifacts(messages, ['library'], ['target-one'])
