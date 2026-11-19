"""Certify why local regrouping of the unchanged SH039 proposal cannot contain its archive."""
from fractions import Fraction
import json
import math
import sys

import numpy as np
from scipy.spatial import ConvexHull

import measure_vulture_mk3_fit as measure
from prepare_vulture_mk3_studio import DETAIL, NAMES, require


def exact_axis_bound(axis, point, maximum, scale):
    """Strict whole-candidate support-plane gap, excluding its 20 mm rounding."""
    squared=measure.fit.dot(axis,axis)
    gap=measure.fit.dot(axis,point)-maximum
    require(squared>0 and gap>0,'Invalid outside projection')
    # Integer square-root lower bound on the unrounded distance in millimetres.
    lower=math.isqrt((gap*1000)**2//(scale**2*squared))-20
    while lower>0 and gap**2<=((Fraction(lower,1000)+Fraction(1,50))*scale)**2*squared:
        lower-=1
    return lower


def interior_witness(vertices,faces,candidate,vertex,face_index,axis):
    face=list(map(int,faces[face_index])); require(vertex in face,'Witness is not in original native face')
    weights=[998 if i==vertex else 1 for i in face]
    point=[sum(Fraction(float(vertices[v][a]))*w for v,w in zip(face,weights,strict=True))/1000 for a in range(3)]
    bound=max(Fraction(float(p[0]))*axis[0] for p in candidate)+Fraction(1,50)
    gap=point[0]*axis[0]-bound
    require(gap>Fraction(17,1000),'Original triangle interior no longer outside the whole possible support set')
    return {'nativeVertex':vertex,'nativeFace':face_index,'sourceFaceIndices':face,'barycentricNumeratorsOver1000':weights,'pointMetres':list(map(float,point)),'exactOutsideLowerMillimetres':17,'axis':list(axis)}


def blocker(meshes,unions):
    candidate,ids=unions['candidate']; original,original_ids=unions['original']; archive=meshes['archivedNative']; native=archive.vertices
    dv=DETAIL[0]; source_ids=original_ids[dv:]
    require(np.array_equal(original[:dv],candidate[:dv]) and np.array_equal(original_ids[:dv],ids[:dv]),'Changed original detailed geometry or groups')
    require(np.all(ids[:dv][meshes['detailed'].faces]==ids[:dv][meshes['detailed'].faces[:,:1]]),'Detailed source face crosses support')
    # No union of subsets of the same input points can exceed their complete
    # convex hull's support in any direction, even if all subsets were merged.
    hull=ConvexHull(candidate)
    normals=hull.equations[:,:3]
    projected=native @ normals.T+hull.equations[:,3]
    best=projected.argmax(axis=1); distance=projected[np.arange(len(native)),best]-.02
    exact,scale=measure.fit.dyadic(np.concatenate((candidate,native)))
    size=len(candidate); maxima={}; witnesses=[]
    for index in np.flatnonzero(distance>.001):
        axis=tuple(int(v) for v in np.rint(normals[best[index]]*measure.fit.GRID))
        if axis not in maxima:maxima[axis]=max(measure.fit.dot(axis,p) for p in exact[:size])
        lower=exact_axis_bound(axis,exact[size+int(index)],maxima[axis],scale)
        require(lower>0,'Numerical outside nomination was not certified')
        witnesses.append({'archivedNativeVertex':int(index),'sourceGroup':NAMES[int(source_ids[index])],'axisIntegerCoefficients':list(axis),'strictOutsideLowerMillimetres':lower})
    require(bool(witnesses),'Expected original native features outside the complete candidate support')
    groups=sorted({w['sourceGroup'] for w in witnesses})
    missing_panels=[name for name in groups if '-nacelle-panel-' in name]
    # The source's omitted closed local panel components have twelve faces each.
    # Appending the complete omitted features, rather than spoofing support-only
    # vertices or removing structural triangles, cannot fit the remaining four.
    panel_faces=sum(int(np.sum(source_ids[archive.faces[:,0]]==NAMES.index(name))) for name in missing_panels)
    require(len(witnesses)>=100 and len(missing_panels)>0 and panel_faces>4,'Expected multi-feature cap blocker')
    side_interiors=[interior_witness(native,archive.faces,candidate,787,1427,(1,0,0)),interior_witness(native,archive.faces,candidate,1459,2471,(-1,0,0))]
    return {'hull':'SH039','candidate':'SH039-DERIVED-NATIVE-508','state':'NO_GROUP_ONLY_CORRECTION_WITH_UNCHANGED_CANDIDATE_POINTS','detailedSourceFacesContainedByOwnConvexGroups':len(meshes['detailed'].faces),'archivedOriginalCoverage':False,'method':'Every regrouping of existing points lies inside their full convex hull; exact integer separating-axis witnesses include 20 mm rounding. Whole candidate hull used only as an impossibility bound, never as proposed collision geometry.','previousLocalWitnessNativeVertex':1017,'certifiedOutsideGlobalRoundedHullVertexCount':len(witnesses),'affectedSourceGroups':groups,'strongestWitness':max(witnesses,key=lambda w:w['strictOutsideLowerMillimetres']),'strictOriginalTriangleInteriorWitnesses':side_interiors,'missingClosedNacellePanelGroups':len(missing_panels),'completeMissingPanelSourceFaces':panel_faces,'candidateFaces':len(meshes['derivedNative'].faces),'faceCap':512,'remainingFaces':512-len(meshes['derivedNative'].faces),'completePanelAdditionWouldExceedCap':panel_faces>512-len(meshes['derivedNative'].faces),'blockerLimit':'Proves no local regrouping of this candidate point set can work. Does not prove that every possible independently redesigned <=512-face native mesh is infeasible. A new approved topology or separately validated collision support stream is required.','outsideWitnesses':witnesses}


if __name__=='__main__':
    sys.stdout.buffer.write((json.dumps(blocker(*measure.load()),indent=2)+'\n').encode('utf-8'))
