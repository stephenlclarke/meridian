"""Export authenticated original input triangles only, before any hull construction."""
import argparse
from fractions import Fraction
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

HEAD = '0c2677b5da76d00155e69ad0495a0413dbf38e48'
SOURCE_SHA = 'd2d6b741f5f022a714da83640132b3ecf3b429ca82dfa74146e682ac0b5667ee'


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def export(project, binding_path, destination):
    binding_bytes = binding_path.read_bytes()
    if hashlib.sha256(binding_bytes).hexdigest() != SOURCE_SHA:
        raise ValueError('Expected sealed source binding')
    binding = json.loads(binding_bytes)
    if binding['sourceCommit'] != HEAD:
        raise ValueError('Expected sealed geometry revision')
    rows = []
    for ident in ('SH010', 'SH011', 'SH016', 'SH017', 'SH166'):
        blueprint_path = 'docs/fleet-physical/' + ident + '-surface-union-blueprint.json'
        blueprint_bytes = (project / blueprint_path).read_bytes()
        if hashlib.sha256(blueprint_bytes).hexdigest() != binding['files'][blueprint_path]:
            raise ValueError('Changed blueprint')
        plan = json.loads(blueprint_bytes)
        recipe_path = plan['recipe']['path']
        recipe_bytes = (project / recipe_path).read_bytes()
        if hashlib.sha256(recipe_bytes).hexdigest() != binding['files'][recipe_path]:
            raise ValueError('Changed recipe')
        inputs = {}
        for key, pin in plan['inputs'].items():
            data = (project / pin['path']).read_bytes()
            if len(data) != pin['bytes'] or hashlib.sha256(data).hexdigest() != pin['sha256']:
                raise ValueError('Changed authenticated original input')
            inputs[key] = data
        recipe = load(project / recipe_path, 'bound_' + ident)
        source, ownership = recipe.authenticate_inputs(**inputs)
        model = source['nativeDesign']
        encoded = []
        for part in ownership['models']['nativeDesign']['parts']:
            first = part['faceStart']
            for local, index in enumerate(range(first, first + part['faceCount'])):
                indices = model['faces'][index]
                texture = dict(model['faceTextures'][index])
                uv = texture.pop('uv')
                style = json.dumps({'material': model['faceMaterials'][index], 'texture': texture},
                                   sort_keys=True, separators=(',', ':')).encode().hex()
                numbers = []
                for point, texcoord in zip(indices, uv, strict=True):
                    numbers.extend(Fraction(str(value)) for value in model['vertices'][point] + texcoord)
                fields = [str(part['index']), str(local), style]
                fields.extend(f'{value.numerator}/{value.denominator}' for value in numbers)
                encoded.append(' '.join(fields))
        payload = ('meridian-exact-facets-v1 ' + str(len(encoded)) + '\n' + '\n'.join(encoded) + '\n').encode()
        (destination / (ident + '.facets')).write_bytes(payload)
        rows.append({'id': ident, 'sourceRevision': HEAD, 'sourceBindingSHA256': SOURCE_SHA,
                     'components': len(ownership['models']['nativeDesign']['parts']),
                     'sourceTriangles': len(encoded), 'exportSHA256': hashlib.sha256(payload).hexdigest(),
                     'blueprintSHA256': hashlib.sha256(blueprint_bytes).hexdigest(),
                     'recipeSHA256': hashlib.sha256(recipe_bytes).hexdigest(),
                     'allEightInputPins': plan['inputs'], 'constructorCalled': False})
    (destination / 'ORIGINAL-BINDINGS.json').write_text(json.dumps(rows, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--source-binding', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    export(args.project, args.source_binding, args.out)
