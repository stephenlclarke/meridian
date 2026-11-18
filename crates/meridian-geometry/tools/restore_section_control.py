"""Restore only the bounded cfg/import metadata on the physical test control."""
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]


def restore_section_control(name, source):
    if name != 'source.rs':
        return source
    test_import = '#[cfg(test)]\nuse crate::exact::{add, scale};\n'
    current_import = 'use crate::exact::{axis, checked, cross, dot, orient, point, project, sub};'
    original_import = 'use crate::exact::{add, axis, checked, cross, dot, orient, point, project, scale, sub};'
    method = '    #[cfg(test)]\n    pub(crate) fn section('
    assert source.count(test_import) == source.count(current_import) == source.count(method) == 1
    result = source.replace(test_import, '', 1).replace(current_import, original_import, 1).replace(method, '    pub(crate) fn section(', 1)
    assert result == (CRATE / 'fixtures/section-control-reference/before-source.rs').read_text()
    return result
