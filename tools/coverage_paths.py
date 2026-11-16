"""Bind Python coverage filenames to real source files across Cargo analysis modules."""
import argparse
from pathlib import Path
import xml.etree.ElementTree as ET


def source_roots(root: Path, report: ET.Element) -> list[Path]:
    roots = [root]
    for source in report.findall('./sources/source'):
        candidate = (root / (source.text or '.')).resolve(strict=True)
        if not candidate.is_relative_to(root) or not candidate.is_dir():
            raise ValueError('Coverage source directory is outside the project')
        roots.append(candidate)
    return roots


def resolve_file(root: Path, roots: list[Path], filename: str) -> Path:
    if not filename or '..' in Path(filename).parts:
        raise ValueError('Invalid coverage filename')
    candidates = {(base / filename).resolve() for base in roots}
    matches = {p for p in candidates if p.is_file() and p.is_relative_to(root)}
    if len(matches) != 1:
        raise ValueError(f'Coverage filename is absent or ambiguous: {filename}')
    return matches.pop()


def normalize(root: Path, source: Path, destination: Path) -> list[str]:
    root = root.resolve(strict=True)
    if source.resolve() == destination.resolve():
        raise ValueError('Keep the original coverage report unchanged')
    report = ET.parse(source)
    roots = source_roots(root, report.getroot())
    classes = report.findall('.//class')
    if not classes:
        raise ValueError('Coverage report has no source classes')
    bindings = [(row, resolve_file(root, roots, row.get('filename', ''))) for row in classes]
    for row, path in bindings:
        row.set('filename', path.relative_to(root).as_posix())
    sources = report.find('./sources')
    if sources is None:
        sources = ET.SubElement(report.getroot(), 'sources')
    sources.clear()
    ET.SubElement(sources, 'source').text = root.as_posix()
    destination.parent.mkdir(parents=True, exist_ok=True)
    report.write(destination, encoding='utf-8', xml_declaration=True)
    return [path.relative_to(root).as_posix() for _, path in bindings]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--input', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args(argv)
    paths = normalize(args.root, args.input, args.output)
    print(f'Bound {len(paths)} coverage source files; line and branch measurements unchanged')


if __name__ == '__main__':
    main()
