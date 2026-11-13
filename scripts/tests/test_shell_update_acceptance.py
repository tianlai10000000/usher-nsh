"""Verify whole-app acceptance evidence, actual archive contents and restart cleanup ownership."""
import copy
import fcntl
import importlib.util
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import xml.etree.ElementTree as ET
import unittest
from unittest.mock import MagicMock, Mock, patch

spec = importlib.util.spec_from_file_location('shell_acceptance', Path(__file__).parents[1] / 'shell_update_acceptance.py')
shell = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shell)


class ShellAcceptanceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def app(self):
        app = self.root / 'Original.app'
        binary = app / 'Contents/MacOS/app'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'executable fixture')
        binary.chmod(0o755)
        metadata = app / 'Contents/Resources/nsh/app.json'
        metadata.parent.mkdir(parents=True)
        metadata.write_text(json.dumps({'id': 'acceptance.unique', 'version': '2.0.0'}))
        (binary.parent / 'alias').symlink_to('app')
        return app

    def test_archive_contains_real_bundle_bytes_modes_and_links(self):
        app = self.app()
        archive = self.root / 'release.tar.gz'
        shell.archive_app(app, archive)
        with tarfile.open(archive) as contents:
            binary = contents.getmember('Original.app/Contents/MacOS/app')
            self.assertEqual(binary.mode, 0o755)
            self.assertEqual(contents.extractfile(binary).read(), b'executable fixture')
            self.assertEqual(contents.getmember('Original.app/Contents/MacOS/alias').linkname, 'app')
            self.assertEqual({p.name.split('/')[0] for p in contents}, {'Original.app'})

    def test_build_variants_use_distinct_explicit_native_configuration(self):
        for appkit in (False, True):
            output = self.root / str(appkit)
            output.mkdir()
            (output / 'Fixture.app').mkdir()
            with patch.object(shell.sim, 'run') as run, patch.object(shell.sim, 'validate_app'):
                shell.build_app(self.root, output, 'acceptance.unique', 10, appkit=appkit)
            self.assertEqual(run.call_args_list[0].kwargs['env']['USHER_MACOS_APPKIT'], str(int(appkit)))
            self.assertEqual(run.call_args_list[1].args[0][:4], ['codesign', '--verify', '--deep', '--strict'])

    def test_gatekeeper_rejection_requires_a_valid_bundle_and_verified_quarantine(self):
        fixture = {'quarantineAttribute': 'test-quarantine'}
        with patch.object(shell.sim, 'run', side_effect=[Mock(), Mock(), Mock(stdout='test-quarantine\n')]) as run, \
             patch.object(shell.subprocess, 'run', return_value=Mock(returncode=3, stdout='', stderr='app: rejected')) as assess:
            evidence = shell.assess_gatekeeper(self.root / 'Fixture.app', fixture, 10)
        self.assertEqual(evidence['exit_code'], 3)
        self.assertIn('Synthetic', evidence['known_limitation'])
        self.assertEqual(run.call_args_list[0].args[0][:4], ['codesign', '--verify', '--deep', '--strict'])
        self.assertIn('--assess', assess.call_args.args[0])
        self.assertNotIn('--master-disable', assess.call_args.args[0])

    def test_gatekeeper_unexpected_success_or_tool_failure_cannot_count_as_rejection(self):
        for result in (Mock(returncode=0, stdout='accepted', stderr=''),
                       Mock(returncode=1, stdout='', stderr='assessment service unavailable')):
            with self.subTest(code=result.returncode), \
                 patch.object(shell.sim, 'run', return_value=Mock(stdout='fixture')), \
                 patch.object(shell.subprocess, 'run', return_value=result):
                with self.assertRaisesRegex(RuntimeError, 'unexpected Gatekeeper'):
                    shell.assess_gatekeeper(self.root, {'quarantineAttribute': 'fixture'}, 10)

    def test_inventory_detects_overlay_leftovers_bytes_permissions_and_links(self):
        app = self.app()
        original = shell.inventory(app)
        other = self.root / 'Installed.app'
        for change in ('content', 'mode', 'link', 'leftover'):
            with self.subTest(change=change):
                shutil.copytree(app, other, symlinks=True)
                binary = other / 'Contents/MacOS/app'
                if change == 'content':
                    binary.write_bytes(b'corrupted')
                elif change == 'mode':
                    binary.chmod(0o644)
                elif change == 'link':
                    link = binary.parent / 'alias'
                    link.unlink()
                    link.symlink_to('elsewhere')
                else:
                    (other / 'retired.txt').write_text('obsolete')
                self.assertNotEqual(original, shell.inventory(other))
                shutil.rmtree(other)

    def test_restart_fences_old_driver_and_discards_queued_commands(self):
        evidence = shell.Evidence()
        first = evidence.record({'kind': 'boot', 'value': {'version': 'old'}})
        evidence.command('install')
        second = evidence.record({'kind': 'boot', 'value': {'version': 'new'}})
        self.assertNotEqual(first, second)
        evidence.command('check')
        self.assertEqual(evidence.next(first), {})
        with self.assertRaisesRegex(ValueError, 'stale'):
            evidence.record({'kind': 'state', 'session': first, 'value': {'version': 'old'}})
        self.assertEqual(evidence.snapshot(), (second, {'version': 'new'}))
        self.assertEqual(evidence.next(second), {'action': 'check'})
        self.assertEqual(evidence.next(second), {})

    def test_teardown_cannot_deliver_install_to_a_late_restart(self):
        evidence = shell.Evidence()
        session = evidence.record({'kind': 'boot', 'value': {}})
        evidence.command('install')
        evidence.close()
        self.assertEqual(evidence.next(session), {})
        with self.assertRaises(ValueError):
            evidence.record({'kind': 'boot', 'value': {}})
        with self.assertRaises(ValueError):
            evidence.command('install')

    def test_runtime_evidence_rejects_wrong_version_package_and_lost_preferences(self):
        preferences = {'colorMode': 'dark', 'localePreference': 'en', 'autoCheck': False}
        good = {'shell': {'version': '2.0.0', 'build': 3}, 'package': None,
                'settings': {'saved': True, 'colorMode': 'dark', 'localePreference': 'en'},
                'update': {'domain': {'auto_check': False}}}
        shell.check_boot(good, '2.0.0', 3, preferences)
        mutations = [('shell', 'version', '1.0.0'), ('shell', 'build', 1),
                     ('settings', 'saved', False), ('settings', 'colorMode', 'light'),
                     ('settings', 'localePreference', 'zh-Hans')]
        for section, field, value in mutations:
            bad = copy.deepcopy(good)
            bad[section][field] = value
            with self.assertRaises(RuntimeError):
                shell.check_boot(bad, '2.0.0', 3, preferences)
        for bad in (dict(good, package={'version': '2.0.0'}),
                    dict(good, update={'domain': {'auto_check': True}})):
            with self.assertRaises(RuntimeError):
                shell.check_boot(bad, '2.0.0', 3, preferences)

    def test_driver_injection_is_confined_to_the_source_copy(self):
        checkout = self.root / 'checkout'
        checkout.mkdir()
        original = '/** App entry. */\nfunction App() {\n  const updates = createUpdates(appearance);\n}\n'
        (checkout / 'App.tsx').write_text(original)
        (checkout / 'nsh.json').write_text(json.dumps({'id': 'product.app', 'schemes': ['product']}))
        shutil.copytree(shell.ROOT / 'tests/e2e/native', checkout / 'tests/e2e/native')
        keys = {'app_id': 'acceptance.unique', 'manifest_pub': 'test', 'root_pub': 'test'}
        with patch.object(shell, 'ROOT', checkout), patch.object(shell.sim, 'ROOT', checkout):
            source = shell.stage_sources(self.root / 'copy', '2.0.0', 3, keys, 12345)
        self.assertEqual((checkout / 'App.tsx').read_text(), original)
        self.assertIn('driveShellAcceptance(updates)', (source / 'App.tsx').read_text())
        self.assertEqual(json.loads((source / 'nsh.json').read_text())['schemes'], [])
        self.assertEqual(json.loads((source / 'nsh.json').read_text())['id'], 'acceptance.unique')

    def test_inner_faults_precede_manifest_signing_but_transport_fault_follows(self):
        app = self.app()
        original = shell.inventory(app)
        for case in ('wrong-identity', 'invalid-signature', 'archive-tampered'):
            directory = self.root / case
            directory.mkdir()
            captured = []
            def run(command):
                if command[1] == 'update-manifest':
                    archive = Path(command[-1])
                    captured.append(archive.read_bytes())
                    with tarfile.open(archive) as contents:
                        captured.append(json.load(contents.extractfile('Original.app/Contents/Resources/nsh/app.json')))
            with patch.object(shell.sim, 'run', side_effect=run) as native:
                _, archive, pristine = shell.prepare_case(case, directory, app, self.root, {'dir': self.root}, '2.0.0')
            if case == 'archive-tampered':
                self.assertNotEqual(archive.read_bytes(), captured[0])
                self.assertEqual(pristine.read_bytes(), captured[0])
            else:
                self.assertEqual(archive.read_bytes(), captured[0])
                self.assertEqual(any(call.args[0][0] == 'codesign' for call in native.call_args_list), case == 'wrong-identity')
                if case == 'wrong-identity':
                    self.assertEqual(captured[1]['id'], 'acceptance.unique.foreign')
                else:
                    self.assertTrue(captured[1]['acceptanceTampered'])
            self.assertEqual(shell.inventory(app), original)

    def test_restart_cleanup_waits_for_late_helper_and_reaps_child(self):
        clock = [1.0]
        def sleep(seconds):
            clock[0] += seconds
        process = Mock()
        process.poll.return_value = 0
        with patch.object(shell.time, 'monotonic', side_effect=lambda: clock[0]), \
             patch.object(shell.time, 'sleep', side_effect=sleep), \
             patch.object(shell, 'live_processes', side_effect=[[11], [], [12]] + [[]] * 20), \
             patch.object(shell.os, 'kill') as kill:
            shell.stop_owned(self.root / 'fixture.app', 'acceptance.unique', process, Mock(return_value=[]), 0.5, 0.1)
        self.assertEqual([call.args[0] for call in kill.call_args_list], [11, 12])
        process.wait.assert_called()

    def test_setup_failure_reports_all_required_cases_with_driver_source_identity(self):
        report = self.root / 'suite.xml'
        with patch.object(shell.sys, 'argv', ['shell-update', '--junit', str(report)]), \
             patch.object(shell.sys, 'platform', 'unsupported'), patch.object(shell.sim, 'COMMAND_LOG'):
            self.assertEqual(shell.main(), 1)
        cases = ET.parse(report).findall('testcase')
        configured = json.loads((shell.ROOT / 'tests/config.json').read_text())['suites']['shell-update']['runs'][0]
        self.assertEqual({case.get('classname') for case in cases}, set(configured['test_paths']))
        self.assertEqual({case.get('name') for case in cases}, set(configured['required_test_prefixes']))
        self.assertTrue(all(case.find('failure') is not None for case in cases))
        self.assertFalse(list(self.root.glob('shell-update-*/isolated-*')))

    def test_interruption_inside_a_case_retains_its_owned_tree(self):
        report = self.root / 'suite.xml'
        app = self.app()
        def interrupt_cleanup(*args):
            args[-1]['status'] = 'passed'
            raise KeyboardInterrupt('during cleanup')
        socket = MagicMock()
        socket.__enter__.return_value.getsockname.return_value = ('127.0.0.1', 12345)
        with patch.object(shell.sys, 'argv', ['shell-update', '--junit', str(report)]), \
             patch.object(shell.sys, 'platform', 'darwin'), patch.object(shell.platform, 'machine', return_value='arm64'), \
             patch.object(shell.sim, 'COMMAND_LOG'), patch.object(shell.sim, 'run', return_value=Mock(stdout='{"ready":true}')), \
             patch.object(shell.sim, 'make_keys', return_value={}), patch.object(shell.sim.socket, 'socket', return_value=socket), \
             patch.object(shell, 'stage_sources', return_value=self.root), patch.object(shell, 'build_app', return_value=app), \
             patch.object(shell, 'verify_interposer', return_value=[]), \
             patch.object(shell, 'run_case', side_effect=interrupt_cleanup):
            self.assertEqual(shell.main(), 1)
        evidence = json.loads(next(self.root.glob('shell-update-*/report.json')).read_text())
        self.assertTrue(evidence['retained'])
        self.assertTrue(evidence['cases'][0]['cleanup_pending'])
        self.assertEqual(evidence['cases'][0]['status'], 'failed')
        self.assertTrue(Path(evidence['sandbox']).is_dir())
        self.assertTrue(all(case.find('failure') is not None for case in ET.parse(report).findall('testcase')))

    def test_simulator_lock_rejects_a_second_native_owner(self):
        simulation = self.root / 'update-sim'
        with patch.object(shell.sim, 'SIM', simulation), patch.object(shell, 'main', return_value=0) as main:
            with (self.root / 'update-sim.lock').open('a') as owner:
                fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                self.assertEqual(shell.locked_main(), 1)
                main.assert_not_called()
            self.assertEqual(shell.locked_main(), 0)
            main.assert_called_once()

    def test_process_discovery_excludes_other_users_and_the_test_host(self):
        app = (self.root / 'fixture.app').resolve()
        output = f'0 11 {app}/Contents/MacOS/app\n501 12 {app}/Contents/MacOS/app\n501 13 /another/app\n501 99 {app}/test-host\n'
        with patch.object(shell.subprocess, 'check_output', return_value=output), \
             patch.object(shell.os, 'getuid', return_value=501), patch.object(shell.os, 'getpid', return_value=99):
            self.assertEqual(shell.live_processes(app), [12])

    def test_transient_kill_denial_requires_a_later_quiet_interval(self):
        clock = [1.0]
        with patch.object(shell.time, 'monotonic', side_effect=lambda: clock[0]), \
             patch.object(shell.time, 'sleep', side_effect=lambda seconds: clock.__setitem__(0, clock[0] + seconds)), \
             patch.object(shell, 'live_processes', side_effect=[[11]] + [[]] * 20), \
             patch.object(shell.os, 'kill', side_effect=PermissionError('exiting')):
            shell.stop_owned(self.root / 'fixture.app', 'acceptance.unique', None, Mock(return_value=[]), 0.3, 0.1)
        self.assertGreaterEqual(clock[0], 1.4)

    def test_persistent_kill_denial_retains_the_namespace(self):
        clock = [1.0]
        with patch.object(shell.time, 'monotonic', side_effect=lambda: clock[0]), \
             patch.object(shell.time, 'sleep', side_effect=lambda seconds: clock.__setitem__(0, clock[0] + seconds)), \
             patch.object(shell, 'live_processes', return_value=[11]), \
             patch.object(shell.os, 'kill', side_effect=PermissionError('denied')):
            with self.assertRaisesRegex(RuntimeError, r'denied PIDs: \[11\]'):
                shell.stop_owned(self.root / 'fixture.app', 'acceptance.unique', None, Mock(return_value=[]), 0.2, 0.1)

    def test_unstoppable_process_is_reported_instead_of_detaching_data(self):
        clock = [1.0]
        with patch.object(shell.time, 'monotonic', side_effect=lambda: clock[0]), \
             patch.object(shell.time, 'sleep', side_effect=lambda seconds: clock.__setitem__(0, clock[0] + seconds)), \
             patch.object(shell, 'live_processes', return_value=[11]), patch.object(shell.os, 'kill'):
            with self.assertRaisesRegex(RuntimeError, 'retain isolation'):
                shell.stop_owned(self.root / 'fixture.app', 'acceptance.unique', None, Mock(return_value=[]), 0.2, 0.1)


if __name__ == '__main__':
    unittest.main()
