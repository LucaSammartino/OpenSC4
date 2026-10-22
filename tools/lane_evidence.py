"""The launcher's evidence contract: an optional list of readable source file paths."""
from pathlib import Path
import stat


def evidence_error(spec_path, field, value, reason):
    return ValueError(f'{spec_path}: {field}={value!r}: {reason}')


def read_source(path, spec_path, index, value):
    """Keep source read failures actionable, including changes after the preflight."""
    try:
        return path.read_bytes()
    except (OSError, ValueError) as exc:
        raise evidence_error(spec_path, f'evidence[{index}]', value,
                             f'cannot read source file: {exc}') from exc


def preflight(root, spec, spec_path):
    """Resolve against root (never cwd/spec parent); leave copy names and overwrite order alone."""
    values = spec.get('evidence', [])
    if not isinstance(values, list):
        raise evidence_error(spec_path, 'evidence', values, 'expected a list of path strings')
    paths = []
    for index, value in enumerate(values):
        field = f'evidence[{index}]'
        if not isinstance(value, str) or not value:
            raise evidence_error(spec_path, field, value, 'expected a nonempty file path string')
        try:
            path = Path(root) / value
            mode = path.stat().st_mode
        except OSError as exc:
            raise evidence_error(spec_path, field, value, f'cannot access source file: {exc}') from exc
        except ValueError as exc:
            raise evidence_error(spec_path, field, value, f'invalid source path: {exc}') from exc
        if not stat.S_ISREG(mode):
            raise evidence_error(spec_path, field, value, 'expected a regular source file')
        read_source(path, spec_path, index, value)
        paths.append(path)
    return paths
