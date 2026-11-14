"""Protect application state ownership and prevent legacy facade or business dependencies returning."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class StateBoundaries(unittest.TestCase):
    def test_generic_crate_has_no_application_or_nsh_dependencies(self):
        manifest = (ROOT / 'crates/usher-state/Cargo.toml').read_text()
        for dependency in ('usher-nsh', 'usher-update', 'nsh-core'):
            self.assertNotIn(dependency, manifest)
        for source in (ROOT / 'crates/usher-state/src').rglob('*.rs'):
            self.assertNotIn('crate::state::appearance', source.read_text())

    def test_removed_facades_cannot_return(self):
        for name in ('src/settings.rs', 'src/app_state.rs', 'src/style_themes.rs',
                     'src/window_service.rs', 'src/window_session.rs',
                     'src/state/updates/registry.rs',
                     'ui/state/appearance.ts', 'ui/state/projection.ts',
                     'ui/state/contracts.ts', 'ui/state/transport.ts'):
            self.assertFalse((ROOT / name).exists(), name)

    def test_business_update_state_stays_out_of_views(self):
        for source in (ROOT / 'ui/features/updates').glob('*.ts*'):
            text = source.read_text()
            for owned in ('createProjection(', 'createController(', '__usherUpdates'):
                self.assertNotIn(owned, text, str(source))

    def test_settings_file_io_has_one_repository(self):
        model = (ROOT / 'src/state/settings/model.rs').read_text().split('#[cfg(test)]\nmod tests')[0]
        self.assertNotIn('fs::', model)
        self.assertIn('repository::read(', model)
        self.assertIn('repository::write(', model)

    def test_business_domains_do_not_borrow_settings_identity_allocator(self):
        for domain in ('appearance', 'diagnostics', 'updates', 'window'):
            for source in (ROOT / 'src/state' / domain).rglob('*.rs'):
                self.assertNotIn('state::settings::next_store_id', source.read_text())

    def test_theme_rendering_does_not_depend_on_application_assembly(self):
        self.assertFalse((ROOT / 'ui/theme/contracts.ts').exists())
        for source in (ROOT / 'ui/theme').glob('*.ts'):
            self.assertNotRegex(source.read_text(), r"state/app(?:\.ts)?[\"']")


if __name__ == '__main__':
    unittest.main()
