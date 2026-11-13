"""Own macOS test data namespaces without overriding the user's HOME or replacing data."""
from pathlib import Path
import re
import shutil
from collections.abc import Callable


def create_link(path: Path, target: Path) -> None:
    """Create only an absent namespace, including rejecting dangling symlinks."""
    if path.exists() or path.is_symlink():
        raise RuntimeError(f'test namespace already exists: {path}')
    target.mkdir(parents=True)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.symlink_to(target, target_is_directory=True)


def remove_link(path: Path, target: Path) -> None:
    """Remove only our own link; AppKit may already have removed an empty saved state."""
    if not path.exists() and not path.is_symlink():
        return
    if not path.is_symlink() or path.readlink() != target:
        raise RuntimeError(f'test namespace ownership changed: {path}')
    path.unlink()


class AppDataNamespace:
    """Own one scenario's data roots; retain them whenever safe cleanup cannot be proved."""

    def __init__(self, directories: dict, app_id: str, root: Path):
        if not re.fullmatch(r'[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+', app_id):
            raise ValueError('invalid test application ID')
        self.root = root.resolve()
        self.links: list[tuple[Path, Path]] = []
        self.owned_root = False
        self.planned = [(Path(directories[key]) / app_id, self.root / key) for key in ('data', 'cache')]
        self.planned.append((Path(directories['library']) / 'Saved Application State' / (app_id + '.savedState'),
                             self.root / 'saved-state'))

    def create(self) -> None:
        """Record links incrementally so partial acquisition can be cleaned by the owner."""
        self.root.mkdir()
        self.owned_root = True
        for path, target in self.planned:
            create_link(path, target)
            self.links.append((path, target))

    def cleanup(self, is_running: Callable[[], bool]) -> None:
        """Never detach a running app or delete a tree when namespace ownership changed."""
        if not self.owned_root:
            return
        if is_running():
            raise RuntimeError('test app still running; isolation retained')
        for path, target in self.links:
            if (path.exists() or path.is_symlink()) and (not path.is_symlink() or path.readlink() != target):
                raise RuntimeError(f'test namespace ownership changed; isolation retained: {path}')
        for path, target in reversed(self.links):
            remove_link(path, target)
        if self.root.exists():
            shutil.rmtree(self.root)

    def evidence(self) -> dict:
        return {str(path): str(target) for path, target in self.links}
