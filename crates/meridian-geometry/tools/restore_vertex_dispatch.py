"""Restore only the two production joint-dispatch cfg attributes."""
from pathlib import Path
from restore_exact_leaf import restore_exact_leaf

CRATE = Path(__file__).resolve().parents[1]


def restore_vertex_dispatch(name, source):
    source = restore_exact_leaf(name, source)
    if name != 'sections.rs':
        return source
    result = source
    for name in ['collection_candidates', 'collection_with_support']:
        method = '    fn ' + name + '('
        assert result.count(method) == 1
        assert '    #[cfg(test)]\n' + method not in result
        result = result.replace(method, '    #[cfg(test)]\n' + method, 1)
    assert result == (CRATE / 'fixtures/vertex-dispatch-reference/before-sections.rs').read_text()
    return result
