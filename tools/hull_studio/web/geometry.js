// The viewer is a presentation adapter. It never builds or validates an envelope.
export const isVector = value => Array.isArray(value) && value.length === 3 && value.every(Number.isFinite);
export const isRecord = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const rgb = value => Array.isArray(value) && value.length === 3 && value.every(channel => Number.isInteger(channel) && channel >= 0 && channel <= 255);

export function meshGeometry(mesh, scale = 1, materials = [], faceMaterials = []) {
  if (!isRecord(mesh) || !Array.isArray(mesh.vertices) || !Array.isArray(mesh.triangles)) {
    return { vertices: [], faces: [], omitted: 0, available: false };
  }
  if (!Number.isFinite(scale) || scale <= 0) return { vertices: [], faces: [], omitted: 0, available: false };
  const vertices = mesh.vertices.map(vertex => {
    if (!isVector(vertex)) return null;
    const scaled = vertex.map(value => value * scale);
    return isVector(scaled) ? scaled : null;
  });
  let omitted = vertices.filter(vertex => vertex === null).length;
  const faces = [];
  mesh.triangles.forEach((face, index) => {
    if (!Array.isArray(face) || face.length !== 3 || !face.every(i => Number.isInteger(i) && i >= 0 && i < vertices.length && vertices[i]) || new Set(face).size !== 3) {
      omitted++;
      return;
    }
    const color = materials?.[faceMaterials?.[index]]?.rgb;
    faces.push({ indices: face, color: rgb(color) ? color : null });
  });
  return { vertices, faces, omitted, available: true };
}

export function sceneFor(definition, report, meshMode = 'detail') {
  const scale = definition?.metresPerUnit;
  const mesh = definition?.[meshMode];
  // The defined material assignments apply to the detailed faces only. Native material
  // assignments are optional authoring data and are not inferred from the detailed list.
  const materialIndices = meshMode === 'detail' ? definition?.faceMaterials : mesh?.faceMaterials ?? definition?.nativeFaceMaterials;
  const source = meshGeometry(mesh, scale, definition?.materials, materialIndices);
  const proposed = meshGeometry(report?.envelope, 1);
  const envelope = proposed.available && proposed.omitted === 0 && proposed.faces.length > 0 ? proposed : null;
  const suppliedPoints = Array.isArray(report?.supportPoints) ? report.supportPoints : Array.isArray(report?.supportGroups) ? report.supportGroups.flatMap(group => Array.isArray(group?.points) ? group.points : []) : [];
  const supportPoints = suppliedPoints.filter(isVector);
  const paths = Array.isArray(definition?.clearances) ? definition.clearances.filter(path => isRecord(path) && isVector(path.start) && isVector(path.end) && Number.isFinite(path.radiusMetres) && path.radiusMetres >= 0).map(path => ({
    name: typeof path.name === 'string' ? path.name : 'Unnamed path',
    start: path.start,
    end: path.end,
    radius: path.radiusMetres,
    passed: Array.isArray(report?.clearances) ? report.clearances.find(result => result?.name === path.name)?.passed : undefined,
  })) : [];
  const measuredGap = Number.isFinite(report?.fit?.maxMetres) && report.fit.maxMetres >= 0 ? report.fit.maxMetres : null;
  const gap = measuredGap !== null && isVector(report.fit.point) ? { point: report.fit.point, metres: measuredGap } : null;
  if (gap && isVector(report.fit.nearestPoint)) gap.nearestPoint = report.fit.nearestPoint;
  const notices = [];
  if (!definition) notices.push('Load or import a hull definition to inspect its source mesh.');
  else if (!source.available) notices.push('Source mesh or scale is unavailable; edit the definition to preview it.');
  else if (source.omitted) notices.push(`${source.omitted} invalid source records omitted from the preview; build to validate.`);
  if (report && !envelope) notices.push(supportPoints.length ? 'No generated envelope surface supplied; showing support points only.' : 'No generated envelope surface or support points supplied.');
  if (!report && definition) notices.push('Source preview · Build & validate to generate the envelope.');
  if (measuredGap !== null && !gap) notices.push('Largest sampled gap measured; no location supplied by the builder.');
  if (Array.isArray(definition?.clearances) && paths.length < definition.clearances.length) notices.push('Invalid clearance definitions omitted from the preview.');
  return { source, envelope, supportPoints, paths, gap, notices };
}

export function fitScene(scene) {
  const low = [Infinity, Infinity, Infinity];
  const high = [-Infinity, -Infinity, -Infinity];
  const include = (point, radius = 0) => {
    if (!isVector(point)) return;
    for (let i = 0; i < 3; i++) {
      low[i] = Math.min(low[i], point[i] - radius);
      high[i] = Math.max(high[i], point[i] + radius);
    }
  };
  for (const vertex of scene.source.vertices) include(vertex);
  for (const vertex of scene.envelope?.vertices ?? []) include(vertex);
  for (const point of scene.supportPoints) include(point);
  for (const path of scene.paths) { include(path.start, path.radius); include(path.end, path.radius); }
  if (scene.gap) { include(scene.gap.point); include(scene.gap.nearestPoint); }
  if (!low.every(Number.isFinite) || !high.every(Number.isFinite)) return { center: [0, 0, 0], radius: 1, low: [-1, -1, -1], empty: true };
  const center = low.map((value, i) => value / 2 + high[i] / 2);
  const span = low.map((value, i) => high[i] / 2 - value / 2);
  const radius = Math.hypot(...span);
  return { center, radius: Number.isFinite(radius) && radius > 0 ? radius : 1, low, empty: false };
}

export function rotatePoint([x, y, z], { yaw, pitch, roll }) {
  const cy = Math.cos(yaw), sy = Math.sin(yaw);
  const cx = Math.cos(pitch), sx = Math.sin(pitch);
  const cz = Math.cos(roll), sz = Math.sin(roll);
  const a = x * cy + z * sy, c = z * cy - x * sy;
  const b = y * cx - c * sx, d = y * sx + c * cx;
  return [a * cz - b * sz, a * sz + b * cz, d];
}

export function inputNumber(text) {
  if (typeof text !== 'string' || text.trim() === '') return null;
  const value = Number(text);
  return Number.isFinite(value) ? value : null;
}

export function updatedParameter(definition, field, value) {
  if (!isRecord(definition)) return definition;
  if (field === 'maxSpeed') return { ...definition, flight: { ...(isRecord(definition.flight) ? definition.flight : {}), maxSpeed: value } };
  return { ...definition, [field]: value };
}

export function groupIds(definition) {
  const meshes = ['detail', 'native'].map(key => definition?.[key]);
  if (!meshes.every(mesh => Array.isArray(mesh?.groups) && mesh.groups.length === mesh.vertices?.length && mesh.groups.every(group => Number.isSafeInteger(group) && group >= 0))) return null;
  return [...new Set(meshes.flatMap(mesh => mesh.groups))].sort((a, b) => a - b);
}

// Adopt only generated memberships, never other package data. The untouched
// definition remains the source of truth until an edit explicitly needs them.
export function resolvedGroupDefinition(definition, packageData) {
  if (groupIds(definition) !== null || groupIds(packageData) === null) return definition;
  const meshes = ['detail', 'native'];
  if (!meshes.every(key => isRecord(definition?.[key]) && Array.isArray(definition[key].vertices) && definition[key].vertices.length === packageData[key].groups.length)) return definition;
  const resolved = { ...definition };
  for (const key of meshes) {
    if (!Object.hasOwn(definition[key], 'groups')) resolved[key] = { ...definition[key], groups: [...packageData[key].groups] };
  }
  return groupIds(resolved) !== null ? resolved : definition;
}

export function mergeGroups(definition, source, target) {
  const groups = groupIds(definition);
  if (!groups?.includes(source) || !groups.includes(target) || source === target) return definition;
  return { ...definition, ...Object.fromEntries(['detail', 'native'].map(key => [key, { ...definition[key], groups: definition[key].groups.map(id => id === source ? target : id) }])) };
}

export function autoGroups(definition) {
  if (!['detail', 'native'].every(key => isRecord(definition?.[key]))) return definition;
  const meshes = ['detail', 'native'].map(key => {
    const { groups, ...mesh } = definition[key];
    return [key, mesh];
  });
  return { ...definition, ...Object.fromEntries(meshes) };
}

export function editableDefinition(input) {
  const value = isRecord(input?.definition) && !('schema' in input) ? input.definition : input;
  if (!isRecord(value)) throw new Error('The hull definition must be a JSON object.');
  return value;
}

export function canInspect(definition, build, revision) {
  return isRecord(definition) && build?.revision === revision && build.report?.valid === true && isRecord(build.package);
}

export function canExport(definition, build, revision) {
  return canInspect(definition, build, revision) && definition.schema !== 2 && build.package.schema !== 2;
}
