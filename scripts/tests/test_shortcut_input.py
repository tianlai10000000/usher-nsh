"""Verify input permission checks use explicit receipts and never silently authorize or skip failures."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('shortcut_input', Path(__file__).parents[1] / 'shortcut_input.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ShortcutInputTests(unittest.TestCase):
    def check_receipt(self, allowed, request=False):
        def launch(command, **kwargs):
            self.assertEqual(command[0:3], ['open', '-g', '-n'])
            self.assertEqual(command[-2], '--request' if request else '--check')
            Path(command[-1]).write_text(json.dumps({'permitted': allowed}))
        with patch.object(module.subprocess, 'run', side_effect=launch):
            self.assertEqual(module.check(Path('/fixture/input.app'), request), allowed)

    def test_default_check_never_requests_permission(self):
        self.check_receipt(False)

    def test_explicit_request_still_requires_positive_receipt(self):
        self.check_receipt(False, request=True)

    def test_positive_permission_receipt(self):
        self.check_receipt(True)

    def test_missing_receipt_is_launch_failure_not_permission_denial(self):
        with patch.object(module.subprocess, 'run'), patch.object(module.time, 'monotonic', side_effect=[0, float('inf')]):
            with self.assertRaisesRegex(RuntimeError, 'did not return'):
                module.check(Path('/fixture/input.app'))


if __name__ == '__main__':
    unittest.main()
