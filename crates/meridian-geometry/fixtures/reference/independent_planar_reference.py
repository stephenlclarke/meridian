"""Frozen exact reference: exhaustive halfplane partition, no segment sweep."""
from fractions import Fraction as F


def orient(a, b, p):
    return (b[0]-a[0])*(p[1]-a[1])-(b[1]-a[1])*(p[0]-a[0])


def twice_area(points):
    return sum(a[0]*b[1]-a[1]*b[0] for a,b in zip(points,points[1:]+points[:1]))


def clip(points, a, b, sign=1):
    result=[]
    if not points:
        return result
    for p,q in zip(points,points[1:]+points[:1]):
        x,y=sign*orient(a,b,p),sign*orient(a,b,q)
        if x>=0:
            result.append(p)
        if x*y<0:
            t=x/(x-y)
            result.append(tuple(p[i]+t*(q[i]-p[i]) for i in range(2)))
    return result if len(result)>=3 and twice_area(result)!=0 else []


def inside(points,p):
    return all(orient(a,b,p)>=0 for a,b in zip(points,points[1:]+points[:1]))


def intersection(first,second):
    result=first
    for a,b in zip(second,second[1:]+second[:1]):
        result=clip(result,a,b)
    return result


def reference(domain,regions,visible):
    pieces=[domain]
    for region in regions:
        for a,b in zip(region,region[1:]+region[:1]):
            next_pieces=[]
            for piece in pieces:
                for sign in (1,-1):
                    part=clip(piece,a,b,sign)
                    if part:
                        next_pieces.append(part)
            pieces=next_pieces
    kept=[]
    for piece in pieces:
        sample=tuple(sum(p[i] for p in piece)/len(piece) for i in range(2))
        covered=any(inside(region,sample) for region in regions)
        if covered==(visible=='inside'):
            kept.append(piece)
    return kept
