"""Protect packaged routing acceptance isolation, source integrity and observation parsing."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('packaged_routing', Path(__file__).parents[1] / 'packaged_routing_acceptance.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PackagedRoutingTests(unittest.TestCase):
    def test_locked_or_missing_sessions_fail_before_native_acceptance(self):
        for session in ({}, {'ready': False}, {'ready': None}):
            with self.assertRaisesRegex(RuntimeError, 'unlocked'):
                module.require_unlocked(session)
        module.require_unlocked({'ready': True})

    def test_source_staging_preserves_product_and_scheme_and_only_instruments_the_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'checkout'
            root.mkdir()
            manifest = {'id': 'production', 'schemes': ['fixture'], 'updater': {'github': 'fixture'}}
            (root / 'nsh.json').write_text(json.dumps(manifest))
            app = '/** Product entry. */\n  bindNativeRouting(navigation);\n'
            (root / 'App.tsx').write_text(app)
            with patch.object(module, 'ROOT', root), patch.object(module.subprocess, 'check_output', return_value=b'nsh.json\0App.tsx\0deleted.ts\0'):
                staged = module.stage_sources(Path(directory) / 'isolated', 'unique.fixture')
            updated = json.loads((staged / 'nsh.json').read_text())
            self.assertEqual(updated, {'id': 'unique.fixture', 'schemes': ['fixture'], 'rust': str(root)})
            self.assertEqual(json.loads((root / 'nsh.json').read_text()), manifest)
            self.assertEqual((root / 'App.tsx').read_text(), app)
            self.assertTrue((staged / 'App.tsx').read_text().startswith('/** Product entry. */'))
            self.assertIn('observePackagedRouter(navigation)', (staged / 'App.tsx').read_text())

    def test_namespace_creation_never_replaces_existing_data_or_dangling_links(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            link, target = root / 'namespace', root / 'data'
            link.write_text('existing')
            with self.assertRaises(RuntimeError):
                module.create_link(link, target)
            self.assertEqual(link.read_text(), 'existing')
            link.unlink()
            link.symlink_to(root / 'missing')
            with self.assertRaises(RuntimeError):
                module.create_link(link, target)
            self.assertFalse(target.exists())

    def test_cleanup_removes_only_owned_links_and_preserves_target_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            link, target = root / 'namespace', root / 'data'
            module.create_link(link, target)
            (link / 'settings').write_text('isolated')
            module.remove_link(link, target)
            self.assertFalse(link.is_symlink())
            module.remove_link(link, target)
            self.assertEqual((target / 'settings').read_text(), 'isolated')
            link.symlink_to(root / 'replacement')
            with self.assertRaises(RuntimeError):
                module.remove_link(link, target)
            self.assertTrue(link.is_symlink())

    def test_incomplete_log_entries_do_not_count_as_navigation_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'app.log'
            self.assertEqual(module.route_events(path, 'MARKER:'), [])
            path.write_text('noise\ninfo MARKER:{"route":"settings","pending":null}\ninfo MARKER:{"route":')
            self.assertEqual(module.route_events(path, 'MARKER:'), [{'route': 'settings', 'pending': None}])


if __name__ == '__main__':
    unittest.main()
