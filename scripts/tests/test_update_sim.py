"""Verify update simulation identity, real data isolation and failure-safe process cleanup."""
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('update_sim', Path(__file__).parents[1] / 'update-sim/sim.py')
sim = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sim)
from macos_test_data import AppDataNamespace


class UpdateSimulationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.directories = {key: str(self.root / key) for key in ('data', 'cache', 'library')}
        self.identity = 'test.fixture.unique'

    def namespace(self):
        return AppDataNamespace(self.directories, self.identity, self.root / 'runtime')

    def app(self, body='import time; time.sleep(30)'):
        app = self.root / 'fixture.app'
        binary = app / 'Contents/MacOS/probe'
        binary.parent.mkdir(parents=True)
        binary.write_text(f'#!{sys.executable}\n{body}\n')
        binary.chmod(0o755)
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
            'CFBundleIdentifier': self.identity, 'CFBundleExecutable': 'probe'}))
        return app

    def test_namespace_writes_only_to_owned_roots_and_cleanup_is_idempotent(self):
        namespace = self.namespace()
        namespace.create()
        for link, target in namespace.links:
            (link / 'evidence').write_text('isolated')
            self.assertEqual((target / 'evidence').read_text(), 'isolated')
        namespace.cleanup(lambda: False)
        namespace.cleanup(lambda: False)
        self.assertFalse(namespace.root.exists())
        self.assertTrue(all(not path.is_symlink() for path, _ in namespace.links))

    def test_partial_creation_preserves_existing_data_and_dangling_link(self):
        for dangling in (False, True):
            with self.subTest(dangling=dangling):
                namespace = self.namespace()
                existing = Path(self.directories['cache']) / self.identity
                existing.parent.mkdir(exist_ok=True)
                if dangling:
                    existing.symlink_to(self.root / 'missing')
                else:
                    existing.write_text('user-data')
                with self.assertRaisesRegex(RuntimeError, 'already exists'):
                    namespace.create()
                namespace.cleanup(lambda: False)
                self.assertFalse((Path(self.directories['data']) / self.identity).is_symlink())
                if dangling:
                    self.assertTrue(existing.is_symlink())
                else:
                    self.assertEqual(existing.read_text(), 'user-data')
                existing.unlink()

    def test_preexisting_runtime_is_not_acquired_or_deleted(self):
        namespace = self.namespace()
        namespace.root.mkdir()
        (namespace.root / 'user').write_text('keep')
        with self.assertRaises(FileExistsError):
            namespace.create()
        namespace.cleanup(lambda: False)
        self.assertEqual((namespace.root / 'user').read_text(), 'keep')

    def test_running_app_or_failed_process_query_retains_all_isolation(self):
        namespace = self.namespace()
        namespace.create()
        with self.assertRaisesRegex(RuntimeError, 'still running'):
            namespace.cleanup(lambda: True)
        with self.assertRaisesRegex(OSError, 'query failed'):
            namespace.cleanup(Mock(side_effect=OSError('query failed')))
        self.assertTrue(all(path.is_symlink() for path, _ in namespace.links))
        namespace.cleanup(lambda: False)

    def test_changed_ownership_prevents_detachment_or_tree_deletion(self):
        namespace = self.namespace()
        namespace.create()
        path, target = namespace.links[-1]
        path.unlink()
        path.symlink_to(self.root / 'replacement')
        with self.assertRaisesRegex(RuntimeError, 'ownership changed'):
            namespace.cleanup(lambda: False)
        self.assertTrue(namespace.root.exists())
        self.assertTrue(namespace.links[0][0].is_symlink())
        self.assertEqual(path.readlink(), self.root / 'replacement')
        path.unlink()
        path.symlink_to(target)
        namespace.cleanup(lambda: False)

    def test_removed_saved_state_is_accepted_during_cleanup(self):
        namespace = self.namespace()
        namespace.create()
        namespace.links[-1][0].unlink()
        namespace.cleanup(lambda: False)
        self.assertFalse(namespace.root.exists())

    def test_identity_cannot_escape_namespace(self):
        for identity in ('../product', '/product', 'test/fixture', ''):
            with self.assertRaises(ValueError):
                AppDataNamespace(self.directories, identity, self.root / 'runtime')

    def test_project_copy_keeps_product_untouched_and_removes_public_scheme(self):
        checkout = self.root / 'checkout'
        checkout.mkdir()
        manifest = {'id': 'product.app', 'schemes': ['product'], 'rust': '.', 'version': '1', 'build': 1}
        (checkout / 'nsh.json').write_text(json.dumps(manifest))
        (checkout / 'App.tsx').write_text('/** Original app. */')
        keys = {'app_id': self.identity, 'manifest_pub': 'test-key', 'root_pub': 'test-root'}
        with patch.object(sim, 'ROOT', checkout):
            project = sim.project_copy(self.root / 'copy', '2', 2, keys, 'http://localhost/latest.json')
        actual = json.loads((project / 'nsh.json').read_text())
        self.assertEqual(actual['id'], self.identity)
        self.assertEqual(actual['rust'], str(checkout))
        self.assertNotIn('schemes', actual)
        self.assertEqual(json.loads((checkout / 'nsh.json').read_text()), manifest)

    def test_product_app_or_url_handler_cannot_be_launched(self):
        app = self.app()
        with self.assertRaisesRegex(RuntimeError, 'identity'):
            sim.validate_app(app, 'different.test')
        info = app / 'Contents/Info.plist'
        value = plistlib.loads(info.read_bytes())
        value['CFBundleURLTypes'] = [{'CFBundleURLSchemes': ['product']}]
        info.write_bytes(plistlib.dumps(value))
        with self.assertRaisesRegex(RuntimeError, 'registration'):
            sim.validate_app(app, self.identity)

    def test_real_child_inherits_home_and_is_reaped_before_data_cleanup(self):
        app = self.app('import os, time; print(os.environ.get("HOME"), flush=True); time.sleep(30)')
        process = sim.AppProcess(app, self.identity, self.root / 'app.log', 5)
        process.start()
        try:
            deadline = time.monotonic() + 5
            while not process.log.read_text() and time.monotonic() < deadline:
                time.sleep(0.01)
        finally:
            process.stop()
        self.assertIsNotNone(process.process.returncode)
        self.assertEqual(process.log.read_text().strip(), os.environ.get('HOME', 'None'))
        process.stop()

    def test_premature_exit_is_a_failure_even_with_no_data_changes(self):
        app = self.app('raise SystemExit(7)')
        process = sim.AppProcess(app, self.identity, self.root / 'app.log', 5)
        with self.assertRaisesRegex(RuntimeError, 'prematurely with 7'):
            process.launch(5)
        self.assertEqual(process.process.returncode, 7)

    def scenario(self, *, fail_build=False, fail_stop=False, interactive=False):
        app = self.app()
        output = self.root / 'reports'
        sandbox = self.root / 'sandbox'
        output.mkdir(); sandbox.mkdir()
        host = lambda command, *args: self.directories if command == 'directories' else []
        process = Mock()
        process.log = self.root / 'app.log'
        process.log.write_text('')
        process.process.poll.return_value = None
        if fail_stop:
            process.stop.side_effect = RuntimeError('stop failed')
        scenario = {'description': 'fixture', 'expect': []}
        with patch.object(sim, 'AppProcess', return_value=process), \
             patch.object(sim, 'build_artifacts', side_effect=RuntimeError('build failed') if fail_build else None, return_value=sandbox), \
             patch.object(sim, 'start_source', return_value=None), \
             patch.object(sim.time, 'sleep', side_effect=KeyboardInterrupt):
            record = sim.run_scenario('fixture', scenario, {'app_id': self.identity},
                                      {'processTimeoutSeconds': 1, 'port': 0, 'launchSeconds': 1},
                                      app, sandbox, output, host, interactive)
        return record, process

    def test_artifact_failure_cleans_partial_scenario(self):
        record, process = self.scenario(fail_build=True)
        self.assertEqual(record['status'], 'failed')
        self.assertEqual(record['cleanup_errors'], [])
        self.assertFalse(any(Path(path).is_symlink() for path in record['isolated_roots']))
        process.stop.assert_called_once()

    def test_stop_failure_retains_data_and_fails_the_scenario(self):
        record, _ = self.scenario(fail_stop=True)
        self.assertEqual(record['status'], 'failed')
        self.assertEqual(record['cleanup_errors'], ['stop failed'])
        self.assertTrue(all(Path(path).is_symlink() for path in record['isolated_roots']))

    def test_interactive_interrupt_stops_owned_app_and_cleans_without_claiming_pass(self):
        record, process = self.scenario(interactive=True)
        self.assertEqual(record['status'], 'observed')
        self.assertEqual(record['cleanup_errors'], [])
        process.start.assert_called_once()
        process.stop.assert_called_once()
        self.assertFalse(any(Path(path).is_symlink() for path in record['isolated_roots']))

    def test_clean_cannot_remove_another_active_runs_cache(self):
        cache = self.root / 'cache'
        cache.mkdir()
        (cache / 'evidence').write_text('keep')
        with patch.object(sim, 'SIM', cache), (self.root / 'cache.lock').open('a') as lock:
            sim.fcntl.flock(lock, sim.fcntl.LOCK_EX | sim.fcntl.LOCK_NB)
            with self.assertRaises(SystemExit) as error, patch('sys.stderr'):
                sim.main(['clean'])
            self.assertNotEqual(error.exception.code, 0)
        self.assertEqual((cache / 'evidence').read_text(), 'keep')

    def test_empty_pending_or_failed_runs_cannot_be_reported_as_success(self):
        for records in ([], [{'status': 'pending'}], [{'status': 'passed'}, {'status': 'failed'}]):
            self.assertEqual(sim.summarize(records)['status'], 'failed')
        result = sim.summarize([{'status': 'passed'}, {'status': 'pending'}])
        self.assertEqual(result, {'passed': 1, 'pending': 1, 'failed': 0, 'status': 'passed'})


if __name__ == '__main__':
    unittest.main()
