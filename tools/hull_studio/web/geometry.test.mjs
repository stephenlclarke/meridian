import test from 'node:test';
import assert from 'node:assert/strict';
import { canExport, editableDefinition, fitScene, inputNumber, meshGeometry, rotatePoint, sceneFor, updatedParameter } from './geometry.js';

const definition = {
  schema: 1, id: 'example', name: 'Example hull', metresPerUnit: 2, roundingMetres: 0.01,
  detail: { vertices: [[1, 2, 3], [0, 0, 0], [1, 0, 0]], triangles: [[0, 1, 2]], groups: [7, 7, 7] },
  native: { vertices: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], triangles: [[0, 1, 2]] },
  materials: [{ name: 'Slate', rgb: [10, 20, 30] }], faceMaterials: [0],
  supports: { intentional: true }, engineering: null, flight: { maxSpeed: null, other: 'preserve' },
  clearances: [{ name: 'Gap', start: [5, 3, 2], end: [8, 3, 2], radiusMetres: 0.1 }], provenance: { note: 'original' }, custom: 'retained',
};

test('source units scale once while backend envelope, clearances and measured gap stay in metres', () => {
  const report = { valid: true, envelope: { vertices: [[2, 4, 6], [0, 0, 0], [2, 0, 0]], triangles: [[0, 1, 2]] }, supportPoints: [[2, 4, 6]], clearances: [{ name: 'Gap', passed: true }], fit: { maxMetres: 0.037, point: [2, 4, 6], method: 'Measured' } };
  const scene = sceneFor(definition, report);
  assert.deepEqual(scene.source.vertices[0], [2, 4, 6]);
  assert.deepEqual(scene.envelope.vertices[0], [2, 4, 6]);
  assert.deepEqual(scene.supportPoints, [[2, 4, 6]]);
  assert.deepEqual(scene.paths[0].start, [5, 3, 2]);
  assert.equal(scene.paths[0].radius, 0.1);
  assert.equal(scene.paths[0].passed, true);
  assert.deepEqual(scene.gap, { point: [2, 4, 6], metres: 0.037 });
  assert.deepEqual(scene.source.faces[0].color, [10, 20, 30]);
  assert.equal(sceneFor(definition, report, 'native').source.faces[0].color, null);
});

test('missing or invalid generated envelope shows only supplied points, never an inferred surface', () => {
  const report = { valid: true, supportPoints: [[1, 2, 3]], fit: { maxMetres: 0.1 } };
  const scene = sceneFor(definition, report);
  assert.equal(scene.envelope, null);
  assert.deepEqual(scene.supportPoints, [[1, 2, 3]]);
  assert.equal(scene.gap, null);
  assert.match(scene.notices.join(' '), /support points only/);
  assert.match(scene.notices.join(' '), /no location supplied/);
  const invalid = sceneFor(definition, { ...report, envelope: { vertices: [[1, 2, 3]], triangles: [[0, 1, 2]] } });
  assert.equal(invalid.envelope, null);
  assert.match(invalid.notices.join(' '), /support points only/);
});

test('invalid source records preserve original indices and are reported rather than silently reindexed', () => {
  const mesh = meshGeometry({ vertices: [[0, 0, 0], ['bad', 0, 0], [1, 0, 0], [0, 1, 0]], triangles: [[0, 1, 2], [0, 2, 3]] });
  assert.deepEqual(mesh.vertices, [[0, 0, 0], null, [1, 0, 0], [0, 1, 0]]);
  assert.deepEqual(mesh.faces[0].indices, [0, 2, 3]);
  assert.equal(mesh.omitted, 2);
  assert.equal(sceneFor({ ...definition, metresPerUnit: null }, null).source.available, false);
});

test('parameter edits preserve all other input fields and nullable max speed', () => {
  const edited = updatedParameter(definition, 'maxSpeed', 40);
  assert.equal(edited.flight.maxSpeed, 40);
  assert.equal(edited.flight.other, 'preserve');
  assert.deepEqual(edited.supports, definition.supports);
  assert.deepEqual(edited.provenance, definition.provenance);
  assert.equal(edited.custom, 'retained');
  assert.equal(definition.flight.maxSpeed, null);
  assert.equal(inputNumber(''), null);
  assert.equal(inputNumber('1.25'), 1.25);
  assert.deepEqual(editableDefinition({ definition }), definition);
  assert.throws(() => editableDefinition([]), /JSON object/);
});

test('export requires the current explicitly validated build and its package', () => {
  assert.equal(canExport(definition, { revision: 2, report: { valid: true }, package: definition }, 2), true);
  assert.equal(canExport(definition, { revision: 2, report: { valid: true }, package: definition }, 3), false);
  assert.equal(canExport(definition, { revision: 2, report: { valid: false }, package: definition }, 2), false);
  assert.equal(canExport(definition, { revision: 2, report: { valid: true }, package: null }, 2), false);
});

test('orbit preserves length and fit uses actual world coordinates', () => {
  const point = [2, 4, 6];
  const moved = rotatePoint(point, { yaw: 1, pitch: -0.5, roll: 0.3 });
  assert.ok(Math.abs(Math.hypot(...point) - Math.hypot(...moved)) < 1e-10);
  const fit = fitScene(sceneFor(definition, null));
  assert.equal(fit.empty, false);
  assert.ok(fit.radius > 0);
  assert.ok(fit.center[0] > 0);
});
