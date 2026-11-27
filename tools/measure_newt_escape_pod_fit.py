"""Bound the rounded Newt vessel to both exact original surface domains."""

import hashlib
import json
from pathlib import Path

import numpy as np
import scipy
import trimesh

import measure_goshawk_surface_fit as fit
from prepare_newt_escape_pod_studio import OUTPUT, definition, require


def audit():
    package=definition(); require(package==json.loads(OUTPUT.read_bytes()),'Changed SH040 package')
    scale=package['metresPerUnit']; detail,native=package['detail'],package['native']
    dv=np.asarray(detail['vertices'])*scale; nv=np.asarray(native['vertices'])*scale
    outer=np.unique(nv,axis=0); whole=np.vstack((dv,nv))
    shell=fit.audit_fan(outer)
    # Prove that each detailed decorative point, as well as the shell itself,
    # lies within every exact support plane of the authenticated native hull.
    exact,_=fit.dyadic(np.vstack((outer,dv)))
    native_count=len(outer); inside=exact[native_count:]
    for ids,_ in shell['supportFaces']:
        a,b,c=(exact[int(i)] for i in ids)
        normal=fit.cross(fit.sub(b,a),fit.sub(c,a)); support=fit.dot(normal,a)
        require(all(fit.dot(normal,p)<=support for p in inside),'A detailed point escapes the native vessel support')
    # The exact source point hull is unchanged by contained interior geometry.
    point_ids={}
    for index,p in enumerate(whole): point_ids.setdefault(tuple(p),index)
    shell['sourceIds']=np.asarray([point_ids[tuple(p)] for p in outer])
    meshes={'detailed':(0,trimesh.Trimesh(dv,detail['triangles'],process=False)),
            'native':(len(dv),trimesh.Trimesh(nv,native['triangles'],process=False))}
    _,integer_scale,source_triangles=fit.integer_source(whole,meshes)
    samples=fit.exposed_samples(shell,[shell]); summary={}
    for name,(_,mesh) in meshes.items():
        row,index=fit.worst_sample(mesh,samples[0]); require(row is not None,'Missing actual exposed sample')
        fit.certify_exposed_sample(row,index,samples,shell,[shell],(source_triangles[name],integer_scale))
        summary[name]={'largestSample':row,'fullUpperMillimetres':fit.certified_upper(mesh,shell,25)}
    return {'hull':'SH040','packageSHA256':hashlib.sha256(OUTPUT.read_bytes()).hexdigest(),
            'auditKernelSHA256':hashlib.sha256(Path(fit.__file__).read_bytes()).hexdigest(),
            'scientificVersions':{'numpy':np.__version__,'scipy':scipy.__version__,'trimesh':trimesh.__version__},
            'containment':{'detailedVerticesChecked':len(dv),'nativeUniqueVertices':len(outer),
                           'exactSupportPlanesChecked':len(shell['supportFaces']),'allDetailedContained':True},
            'summary':summary,'scope':'Exact source native convex support contains all decorative interior points. Rounded exposed lower and full piece upper bound actual surfaces, not optical rendering or gameplay admission.'}


if __name__=='__main__': print(json.dumps(audit(),indent=2))
