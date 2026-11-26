"""Generate the exact flight-computer inverse from the frozen integrated source."""
import difflib
import json
import subprocess
import sys

from verify_mule_envelope_source import (
    FLIGHT_COMPUTER_BASE, ROOT, protected_core_pins, sha, undo_flight_computer,
)


def file_inverse(before, after):
    """Record changed byte spans in the integrated file and their original bytes."""
    old = before.splitlines(keepends=True)
    new = after.splitlines(keepends=True)
    offsets = [0]
    for line in new:
        offsets.append(offsets[-1] + len(line))
    matcher = difflib.SequenceMatcher(None, old, new, autojunk=False)
    hunks = [
        {
            'offset': offsets[new_start],
            'before': b''.join(old[old_start:old_end]).decode('utf-8'),
            'after': b''.join(new[new_start:new_end]).decode('utf-8'),
        }
        for tag, old_start, old_end, new_start, new_end in matcher.get_opcodes()
        if tag != 'equal'
    ]
    record = {'beforeSHA256': sha(before), 'afterSHA256': sha(after), 'hunks': hunks}
    if before != after:
        assert undo_flight_computer('source', after, {'source': record}) == before
    return record


def generate(root, base=FLIGHT_COMPUTER_BASE):
    """Compare only the existing protected paths against their immutable Git blobs."""
    prefix = 'crates/meridian-core/src/'
    inverse = {}
    for relative in sorted(protected_core_pins(root)):
        path = prefix + relative
        before = subprocess.check_output(
            ['git', 'cat-file', 'blob', base + ':' + path], cwd=root,
        )
        after = (root / path).read_bytes()
        if before != after:
            inverse[relative] = file_inverse(before, after)
    assert inverse, 'flight computer source must have a protected delta'
    return {'base': base, 'files': inverse}


if __name__ == '__main__':
    # The default is the historical flight-computer source. A new, separately pinned
    # inverse may use its own explicit immutable parent without rewriting old evidence.
    base = sys.argv[1] if len(sys.argv) == 2 else FLIGHT_COMPUTER_BASE
    assert len(sys.argv) <= 2 and len(base) == 40
    sys.stdout.buffer.write((json.dumps(generate(ROOT, base), indent=2) + '\n').encode('utf-8'))
