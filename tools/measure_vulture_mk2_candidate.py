"""Compare the separate SH038 candidate rounded support union to both original meshes."""

import hashlib
import json
import math
from pathlib import Path

import numpy as np
import scipy
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_vulture_mk2_source import ASSETS, authenticated, require
from prepare_vulture_mk2_candidate import CANDIDATE, definition


def audit():
    source,_,_ = authenticated(ASSETS); candidate = definition()
    require(candidate==json.loads(CANDIDATE.read_bytes()),'Changed SH038 candidate')
    scale = candidate['metresPerUnit']; detail,native = source['design'],source['nativeDesign']
    original = np.asarray(detail['vertices']+native['vertices'])*scale
    meshes={'detailed':(0,trimesh.Trimesh(original[:len(detail['vertices'])],detail['faces'],process=False)),
            'native':(len(detail['vertices']),trimesh.Trimesh(original[len(detail['vertices']):],native['faces'],process=False))}
    source_ids={}
    for i,p in enumerate(original): source_ids.setdefault(tuple(p),i)
    names=candidate['provenance']['groupNames']; groups=[set() for _ in names]
    for mesh in (candidate['detail'],candidate['native']):
        for point,group in zip(mesh['vertices'],mesh['groups'],strict=True): groups[group].add(tuple(np.asarray(point)*scale))
    pieces=[]
    for name,group in zip(names,groups,strict=True):
        points=np.asarray(sorted(group))
        try:
            piece=fit.audit_fan(points)
        except (AssertionError,ValueError) as error:
            raise ValueError(f'Cannot certify source-local support {name}') from error
        piece['sourceIds']=np.asarray([source_ids[tuple(p)] for p in points]); pieces.append(piece)
    _,integer_scale,exact=fit.integer_source(original,meshes)
    rows=[]; witnesses={k:[] for k in meshes}
    for name,piece in zip(names,pieces,strict=True):
        samples=fit.exposed_samples(piece,pieces); comparisons={}
        for key,(_,mesh) in meshes.items():
            row,index=fit.worst_sample(mesh,samples[0])
            if row is not None: witnesses[key].append((row,index,samples,piece))
            comparisons[key]={'largestExposedSample':row,'fullRoundedPieceUpperMillimetres':fit.certified_upper(mesh,piece,1000 if key=='detailed' else 1600)}
        rows.append({'group':name,'comparisons':comparisons})
    summary={}
    for key,values in witnesses.items():
        largest=sorted(values,key=lambda r:-r[0]['metres'])[:2]
        require(bool(largest),'No exposed support witness')
        for row,index,samples,piece in largest: fit.certify_exposed_sample(row,index,samples,piece,pieces,(exact[key],integer_scale))
        summary[key]={'largestSample':largest[0][0],'fullUpperMillimetres':max(row['comparisons'][key]['fullRoundedPieceUpperMillimetres'] for row in rows)}
    reduced=trimesh.Trimesh(np.asarray(candidate['native']['vertices'])*scale,candidate['native']['triangles'],process=False)
    comparison={}
    # The visual approximation is distinct from the rounded support bounds.
    def surface_comparison(origin,target):
        _,dist,face=fit.nearest(target,origin.vertices); index=int(np.argmax(dist))
        lower=max(0,int(dist[index]*1000)-1)
        if lower:
            exact,unit=fit.dyadic(np.vstack((origin.vertices[index],target.vertices)))
            target_triangles=[tuple(exact[int(i)+1] for i in f) for f in target.faces]
            fit.verify_lower(exact[0],target_triangles,unit,lower)
        cutoff=100*(math.ceil(float(dist[index])*10)+2)
        upper=fit.certified_upper(target,{'triangles':origin.triangles},cutoff)
        return {'vertex':index,'nearestFace':int(face[index]),'maxVertexSampleMetres':float(dist[index]),
                'certifiedLowerMillimetres':lower,'fullSurfaceUpperMillimetres':upper}
    for key,(_,mesh) in meshes.items():
        comparison[key]={'derivedNativeToOriginal':surface_comparison(reduced,mesh),
                         'originalToDerivedNative':surface_comparison(mesh,reduced),
                         'scope':'Independent bidirectional full surface bounds; shared patch kernel includes conservative extra 20 mm margin'}
    return {'hull':'SH038','candidateSHA256':hashlib.sha256(CANDIDATE.read_bytes()).hexdigest(),
            'auditKernelSHA256':hashlib.sha256(Path(fit.__file__).read_bytes()).hexdigest(),
            'scientificVersions':{'numpy':np.__version__,'scipy':scipy.__version__,'trimesh':trimesh.__version__},
            'summary':summary,'perGroup':rows,'derivedNativeComparison':comparison,
            'scope':'Conservative full rounded piece upper bound covers exposed union; certified exposed sample lower. Derived native comparisons separate.'}


if __name__=='__main__': print(json.dumps(audit(),indent=2))
