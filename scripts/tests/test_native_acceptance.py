"""Verify native acceptance staging cannot mutate or reuse the application's data/configuration."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('native_acceptance', Path(__file__).parents[1] / 'native_acceptance.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class NativeIsolationTests(unittest.TestCase):
    def test_staging_copies_sources_and_removes_release_updater_configuration(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'checkout'
            root.mkdir()
            for directory in ('ui', 'i18n', 'tests/e2e/native'):
                (root / directory).mkdir(parents=True)
            (root / 'ui/source.ts').write_text('original')
            (root / 'tsconfig.json').write_text('{}')
            original = {'id': 'production', 'rust': '.', 'updater': {'secret': 'fixture'}, 'theme': 'ui/theme.json'}
            (root / 'nsh.json').write_text(json.dumps(original))
            fixture = root / 'tests/e2e/native'
            (fixture / 'config.json').write_text(json.dumps({'appId': 'isolated.fixture'}))
            destination = Path(temporary) / 'isolated'
            destination.mkdir()
            with patch.object(module, 'ROOT', root), patch.object(module, 'FIXTURE', fixture):
                staged = module.isolated_copy(destination)
                config = json.loads((staged / 'nsh.json').read_text())
                self.assertEqual(config['id'], 'isolated.fixture')
                self.assertNotIn('updater', config)
                self.assertEqual(config['rust'], str(root))
                self.assertEqual(config['capabilities'], {'windows': True, 'statusBar': True})
                (staged / 'ui/source.ts').write_text('hot update')
                self.assertEqual((root / 'ui/source.ts').read_text(), 'original')
                self.assertEqual(json.loads((root / 'nsh.json').read_text()), original)
                # A new run must never silently reuse a source/data tree from an older one.
                with self.assertRaises(FileExistsError):
                    module.isolated_copy(destination)


if __name__ == '__main__':
    unittest.main()
