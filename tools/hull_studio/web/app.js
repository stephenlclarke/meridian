import { autoGroups, canExport, canInspect, editableDefinition, groupIds, inputNumber, isRecord, isVector, mergeGroups, resolvedGroupDefinition, updatedParameter } from './geometry.js';
import { HullViewer } from './viewer.js';

const $ = id => document.getElementById(id);
const viewer = new HullViewer($('hull-canvas'), $('geometry-notice'));
const state = { definition: null, revision: 0, build: null, mesh: 'detail', dirty: false, job: null, error: null, status: 'Loading example…', tone: 'busy' };
const fields = [['hull-id', 'id', false], ['hull-name', 'name', false], ['scale', 'metresPerUnit', true], ['rounding', 'roundingMetres', true], ['speed', 'maxSpeed', true]];
let lastScene = [];
let groupHistory = [];
let showroomAvailable = false;

const numberText = value => Number.isFinite(value) ? value.toLocaleString('en-US', { maximumFractionDigits: 8 }) : '—';
const text = (id, value) => { $(id).textContent = value; };
const fieldValue = (definition, field) => field === 'maxSpeed' ? definition?.flight?.maxSpeed : definition?.[field];
const currentBuild = () => state.build?.revision === state.revision ? state.build : null;

function cancelJob() {
  if (state.job) state.job.controller.abort();
  state.job = null;
}

function beginJob(kind, status) {
  cancelJob();
  const job = { kind, revision: state.revision, controller: new AbortController() };
  state.job = job;
  if (kind === 'build') state.build = null;
  state.status = status;
  state.tone = 'busy';
  state.error = null;
  render(false);
  return job;
}

function jobCurrent(job) { return state.job === job && state.revision === job.revision; }

function setDefinition(definition, status, dirty = true) {
  cancelJob();
  groupHistory = [];
  state.definition = editableDefinition(definition);
  state.revision++;
  state.build = null;
  state.error = null;
  state.dirty = dirty;
  state.status = status;
  state.tone = dirty ? 'stale' : 'ready';
  viewer.reset();
  render();
}

function editParameter(field, value) {
  if (!state.definition || fieldValue(state.definition, field) === value) return;
  cancelJob();
  groupHistory = [];
  state.definition = updatedParameter(state.definition, field, value);
  state.revision++;
  state.build = null;
  state.error = null;
  state.dirty = true;
  state.status = 'Definition changed · build required';
  state.tone = 'stale';
  render(false);
}

function editableGroups(definition, build = currentBuild()) {
  return canExport(definition, build, state.revision) ? resolvedGroupDefinition(definition, build.package) : definition;
}

function renderGroups(definition, build) {
  const authored = groupIds(definition);
  const ids = groupIds(editableGroups(definition, build));
  const previous = [$('group-source').value, $('group-target').value];
  for (const [index, key] of ['group-source', 'group-target'].entries()) {
    const select = $(key);
    select.replaceChildren(...(ids ?? []).map(id => new Option(`Group ${id}`, String(id))));
    select.disabled = !ids || ids.length < 2;
    if (ids?.includes(Number(previous[index])) && previous[index] !== '') select.value = previous[index];
    else if (ids?.length > index) select.value = String(ids[index]);
  }
  const count = ids === null ? 'Auto / unset · build to assign components' : `${ids.length} ${authored === null ? 'resolved' : 'authored'} ${ids.length === 1 ? 'group' : 'groups'} across both meshes`;
  text('group-summary', !definition ? 'No groups loaded.' : authored === null && ids !== null ? `${count} · merge to adopt` : count);
  $('auto-groups').disabled = !definition || !['detail', 'native'].some(key => Object.hasOwn(definition[key] ?? {}, 'groups'));
  $('undo-groups').disabled = !groupHistory.length;
  updateMergeButton();
}

function updateMergeButton() {
  $('merge-groups').disabled = $('group-source').disabled || $('group-source').value === $('group-target').value;
}

function changeGroups(next, status, undo = false) {
  if (next === state.definition) return;
  cancelJob();
  if (!undo) groupHistory.push(state.definition);
  state.definition = next;
  state.revision++;
  state.build = null;
  state.error = null;
  state.dirty = true;
  state.status = status;
  state.tone = 'stale';
  render(false);
}

function meshSummary(mesh) {
  if (!Array.isArray(mesh?.vertices) || !Array.isArray(mesh?.triangles)) return '—';
  return `${numberText(mesh.triangles.length)} tris / ${numberText(mesh.vertices.length)} verts`;
}

function countGroups(meshes) {
  if (!meshes.every(mesh => Array.isArray(mesh?.groups) && mesh.groups.length === mesh.vertices?.length && mesh.groups.every(Number.isInteger))) return null;
  return new Set(meshes.flatMap(mesh => mesh.groups)).size;
}

function renderMaterials(definition) {
  const list = $('material-list');
  list.replaceChildren();
  const materials = Array.isArray(definition?.materials) ? definition.materials : [];
  $('visual-details').hidden = materials.length === 0;
  for (const [index, item] of materials.entries()) {
    const li = document.createElement('li');
    const swatch = document.createElement('span');
    swatch.className = 'material-swatch';
    if (Array.isArray(item?.rgb) && item.rgb.length === 3 && item.rgb.every(c => Number.isInteger(c) && c >= 0 && c <= 255)) {
      swatch.style.backgroundColor = `rgb(${item.rgb.join(',')})`;
      swatch.title = `RGB ${item.rgb.join(', ')}`;
    }
    li.append(swatch, document.createTextNode(typeof item?.name === 'string' ? item.name : `Material ${index}`));
    list.append(li);
  }
}

function diagnostic(value, tone = 'info') {
  if (typeof value === 'string') return { message: value, tone };
  if (!isRecord(value)) return { message: String(value), tone };
  const severity = value.severity ?? value.level;
  const classified = severity === 'error' ? 'error' : severity === 'warning' || severity === 'warn' ? 'warning' : severity === 'success' ? 'success' : tone;
  return { message: typeof value.message === 'string' ? value.message : JSON.stringify(value), tone: classified };
}

function renderDiagnostics(report, definition) {
  const list = $('diagnostics-list');
  const items = Array.isArray(report?.diagnostics) ? report.diagnostics.map(item => diagnostic(item, report.valid === false ? 'warning' : 'info')) : [];
  if (typeof report?.viewWarning === 'string') items.push(diagnostic(report.viewWarning, 'warning'));
  if (state.error && !items.some(item => item.message === state.error)) items.unshift(diagnostic(state.error, 'error'));
  if (state.job?.kind === 'build') items.unshift(diagnostic('Building and validating the current source…'));
  else if (!report && !state.error) {
    items.push(diagnostic('Build has not been run yet.'), diagnostic('Validation has not been run yet.'));
    const count = Array.isArray(definition?.clearances) ? definition.clearances.length : 0;
    if (count) items.push(diagnostic(`${count} clearance ${count === 1 ? 'path' : 'paths'} defined; run Build & validate to check clearance.`, 'warning'));
  } else if (report && !items.length) items.push(diagnostic(report.valid === true ? 'The builder returned a valid package with no diagnostics.' : 'The builder did not return a valid package.', report.valid === true ? 'success' : 'warning'));
  list.replaceChildren();
  for (const item of items) {
    const li = document.createElement('li');
    const icon = document.createElement('span');
    icon.className = `diagnostic-icon ${item.tone}`;
    icon.setAttribute('aria-hidden', 'true');
    icon.textContent = item.tone === 'error' || item.tone === 'warning' ? '!' : item.tone === 'success' ? '✓' : 'i';
    li.append(icon, document.createTextNode(item.message));
    list.append(li);
  }
  text('diagnostics-title', `Diagnostics (${items.length})`);
}

function renderReport(build, definition) {
  const report = build?.report;
  const status = $('report-status');
  const building = state.job?.kind === 'build';
  const valid = canInspect(definition, build, state.revision);
  status.className = `report-status ${building ? 'pending' : valid ? 'valid' : report?.valid === false || state.error ? 'invalid' : ''}`;
  text('report-status-text', building ? 'Building / Validating…' : valid ? 'Built / Validated' : report?.valid === false ? 'Validation failed' : report ? 'Not validated' : 'Not built / Not validated');
  const groups = report?.sourceGroups ?? report?.sourceGroupCount ?? countGroups([build?.package?.detail ?? definition?.detail, build?.package?.native ?? definition?.native]);
  text('source-groups', Number.isFinite(groups) ? numberText(groups) : definition ? 'Auto / unset' : '—');
  const envelopeGroups = report?.envelopeGroups ?? report?.envelopeGroupCount ?? report?.envelope?.groupCount ?? (Array.isArray(report?.supportGroups) ? report.supportGroups.length : null);
  text('envelope-groups', Number.isFinite(envelopeGroups) ? numberText(envelopeGroups) : '—');
  const gap = report?.fit?.maxMetres;
  const measured = Number.isFinite(gap) && gap >= 0;
  text('maximum-gap', measured ? `${gap.toFixed(3)} m` : 'Not measured');
  const details = [];
  if (measured && typeof report.fit.method === 'string') details.push(report.fit.method);
  if (Number.isFinite(report?.fit?.upperBoundMetres)) details.push(`Verified upper bound ${numberText(report.fit.upperBoundMetres)} m`);
  if (Number.isFinite(report?.fit?.samples)) details.push(`${numberText(report.fit.samples)} samples`);
  if (measured && !isVector(report.fit.point)) details.push('No measured point supplied.');
  $('gap-note').hidden = !details.length;
  text('gap-note', details.join(' · '));
  const clearances = Array.isArray(report?.clearances) ? report.clearances : null;
  const listedClearances = clearances ?? (Array.isArray(definition?.clearances) ? definition.clearances : []);
  const expected = Array.isArray(definition?.clearances) ? definition.clearances.length : 0;
  text('clearance-checks', clearances ? `${clearances.filter(item => item?.passed === true).length} / ${clearances.length}` : expected ? `— / ${expected}` : '—');
  const results = $('clearance-result-list');
  results.replaceChildren();
  $('clearance-results').hidden = !listedClearances.length;
  for (const [index, item] of listedClearances.entries()) {
    const li = document.createElement('li');
    const name = document.createElement('span');
    name.textContent = typeof item?.name === 'string' ? item.name : `Path ${index + 1}`;
    const result = document.createElement('span');
    result.className = item?.passed === true ? 'passed' : item?.passed === false ? 'failed' : 'unknown';
    result.textContent = clearances ? item?.passed === true ? 'Pass' : item?.passed === false ? 'Fail' : 'Unknown' : 'Unchecked';
    li.append(name, result);
    results.append(li);
  }
  renderDiagnostics(report, definition);
}

function renderInspect(build) {
  const selector = $('inspect-select');
  const reportOption = selector.querySelector('[value="report"]');
  const packageOption = selector.querySelector('[value="package"]');
  reportOption.disabled = !build?.report;
  packageOption.disabled = !build?.package;
  $('inspect-options').hidden = !build?.report && !build?.package;
  if (selector.selectedOptions[0]?.disabled) selector.value = 'definition';
  const value = selector.value === 'package' ? build?.package : selector.value === 'report' ? build?.report : state.definition;
  const raw = value ? JSON.stringify(value, null, 2) : 'No definition loaded.';
  const maxPreview = 16000;
  text('json-preview', raw.length > maxPreview ? `${raw.slice(0, maxPreview)}\n… Preview limited to ${numberText(maxPreview)} characters; the complete data is retained.` : raw);
}

function render(syncFields = true) {
  const definition = state.definition;
  const build = currentBuild();
  text('document-title', definition ? (typeof definition.name === 'string' && definition.name.trim() ? definition.name : typeof definition.id === 'string' && definition.id ? definition.id : 'Untitled hull') + (state.dirty ? ' *' : '') : 'No definition');
  for (const [id, key] of fields) {
    const field = $(id);
    field.disabled = !definition;
    if (syncFields) field.value = fieldValue(definition, key) == null ? '' : String(fieldValue(definition, key));
  }
  text('detail-summary', meshSummary(definition?.detail));
  text('native-summary', meshSummary(definition?.native));
  text('material-summary', Array.isArray(definition?.materials) ? numberText(definition.materials.length) : '—');
  text('clearance-summary', Array.isArray(definition?.clearances) ? numberText(definition.clearances.length) : '—');
  renderMaterials(definition);
  renderGroups(definition, build);
  $('save-button').disabled = !definition;
  $('build-button').disabled = !definition || Boolean(state.job);
  $('export-button').disabled = !canExport(definition, build, state.revision) || Boolean(state.job);
  $('showroom-button').hidden = !showroomAvailable;
  $('showroom-button').disabled = !canInspect(definition, build, state.revision) || Boolean(state.job);
  $('load-example').disabled = state.job?.kind === 'example';
  $('quit-button').disabled = Boolean(state.job);
  text('schema-status', definition?.schema == null ? 'Schema —' : `Schema ${definition.schema}${definition.schema === 2 ? ' · inspection only' : ''}`);
  text('status-text', state.status);
  $('state-dot').className = `state-dot ${state.tone}`;
  renderReport(build, definition);
  renderInspect(build);
  const scene = [definition, build?.report, state.mesh];
  if (scene.some((value, index) => value !== lastScene[index])) { viewer.setScene(...scene); lastScene = scene; }
}

async function jsonResponse(response) {
  try { return await response.json(); }
  catch { throw new Error(`The builder returned an unreadable response (HTTP ${response.status}).`); }
}

function finishError(job, error, status = 'Operation failed') {
  if (!jobCurrent(job) || error?.name === 'AbortError') return;
  state.job = null;
  state.error = error instanceof Error ? error.message : String(error);
  state.status = status;
  state.tone = 'error';
  render(false);
}

async function loadExample(automatic = false) {
  if (!automatic && state.dirty && !window.confirm('Replace the current unsaved definition with the example?')) return;
  const job = beginJob('example', 'Loading example…');
  try {
    const response = await fetch('/api/example', { signal: job.controller.signal, headers: { Accept: 'application/json' } });
    const payload = await jsonResponse(response);
    if (!response.ok) throw new Error(payload?.error || `Could not load example (HTTP ${response.status}).`);
    if (jobCurrent(job)) setDefinition(payload, 'Example loaded · ready to build', false);
  } catch (error) { finishError(job, error, 'Example unavailable · import or edit JSON'); }
}

async function importFile(file) {
  if (!file) return;
  if (state.dirty && !window.confirm('Replace the current unsaved definition with this file?')) return;
  const job = beginJob('import', 'Reading definition…');
  try {
    const definition = editableDefinition(JSON.parse(await file.text()));
    if (jobCurrent(job)) setDefinition(definition, 'Definition imported · build required', false);
  } catch (error) { finishError(job, error, 'Import failed · current definition retained'); }
}

async function buildDefinition() {
  if (!state.definition || state.job) return;
  const job = beginJob('build', 'Building and validating…');
  try {
    const response = await fetch('/api/build', { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' }, body: JSON.stringify({ definition: state.definition }), signal: job.controller.signal });
    const payload = await jsonResponse(response);
    if (!jobCurrent(job)) return;
    const report = isRecord(payload?.report) ? payload.report : null;
    state.job = null;
    if (!response.ok) {
      state.error = typeof payload?.error === 'string' ? payload.error : `Build refused (HTTP ${response.status}).`;
      state.build = { revision: state.revision, package: null, report: report ? { ...report, valid: false } : { valid: false, diagnostics: [] } };
      state.status = 'Build refused · review diagnostics';
      state.tone = 'error';
    } else if (!report) {
      state.error = 'The builder did not return a validation report.';
      state.build = null;
      state.status = 'Build response incomplete';
      state.tone = 'error';
    } else {
      state.build = { revision: state.revision, package: isRecord(payload.package) ? payload.package : null, report };
      const valid = canInspect(state.definition, state.build, state.revision);
      if (valid) {
        state.status = state.definition.schema === 2 ? "Archive validated · inspection only" : "Validated · ready to export";
      } else {
        state.status = "Build not validated · review diagnostics";
      }
      state.tone = valid ? 'ready' : 'error';
      if (report.valid === true && !state.build.package) state.error = 'The report was valid, but no package was returned.';
    }
    render(false);
  } catch (error) { finishError(job, error, 'Build failed · try again'); }
}

function safeFilename(value, fallback) {
  const basename = typeof value === 'string' ? value.split(/[/\\]/).pop() : '';
  const cleaned = basename.replace(/[^a-zA-Z0-9_.-]/g, '_').replace(/^\.+/, '').slice(0, 120);
  return cleaned || fallback;
}

function download(blob, name) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function saveDefinition() {
  if (!state.definition) return;
  const name = safeFilename(state.definition.id, 'hull');
  download(new Blob([JSON.stringify(state.definition, null, 2) + '\n'], { type: 'application/json' }), `${name}-definition.json`);
  state.dirty = false;
  state.status = 'Definition saved';
  state.tone = 'ready';
  render(false);
}

async function exportPackage() {
  if (!canExport(state.definition, currentBuild(), state.revision) || state.job) return;
  const job = beginJob('export', 'Exporting validated package…');
  try {
    const response = await fetch('/api/export', { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' }, body: JSON.stringify({ definition: state.definition }), signal: job.controller.signal });
    if (!response.ok) { const payload = await jsonResponse(response); throw new Error(payload?.error || `Export refused (HTTP ${response.status}).`); }
    const blob = await response.blob();
    const exported = editableDefinition(JSON.parse(await blob.text()));
    if (exported.schema !== 1 || exported.id !== state.definition.id || !Array.isArray(exported.detail?.triangles) || !Array.isArray(exported.native?.triangles)) throw new Error('Export response was not the expected validated hull package.');
    if (!jobCurrent(job)) return;
    const header = response.headers.get('Content-Disposition') || '';
    const encoded = header.match(/filename\*=UTF-8''([^;]+)/i)?.[1];
    const plain = header.match(/filename\s*=\s*(?:"([^"]+)"|([^;]+))/i);
    let name = plain?.[1] || plain?.[2];
    if (encoded) { try { name = decodeURIComponent(encoded.trim()); } catch { /* Use regular filename. */ } }
    download(blob, safeFilename(name, `${safeFilename(state.definition.id, 'hull')}.json`));
    state.job = null;
    state.status = 'Validated package exported';
    state.tone = 'ready';
    render(false);
  } catch (error) { finishError(job, error, 'Export refused · rebuild if needed'); }
}

async function discoverCapabilities() {
  try {
    const response = await fetch('/api/capabilities', { headers: { Accept: 'application/json' } });
    showroomAvailable = response.ok && (await jsonResponse(response))?.showroom === true;
  } catch { showroomAvailable = false; }
  render(false);
}

async function openShowroom() {
  if (!showroomAvailable || !canInspect(state.definition, currentBuild(), state.revision) || state.job) return;
  const job = beginJob('showroom', 'Opening isolated game showroom…');
  try {
    const response = await fetch('/api/showroom', { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' }, body: JSON.stringify({ definition: state.definition }), signal: job.controller.signal });
    const payload = await jsonResponse(response);
    if (!jobCurrent(job)) return;
    if (!response.ok || payload?.opened !== true) throw new Error(payload?.error || `Showroom refused (HTTP ${response.status}).`);
    state.job = null;
    state.status = 'Opened in isolated game showroom';
    state.tone = 'ready';
    render(false);
  } catch (error) { finishError(job, error, 'Showroom could not be opened · validated build retained'); }
}

async function quitStudio() {
  if (state.job) return;
  if (state.dirty && !window.confirm('The definition has unsaved changes. Quit Studio without saving?')) return;
  const job = beginJob('quit', 'Closing local Studio…');
  try {
    const response = await fetch('/api/quit', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: '{}', signal: job.controller.signal });
    if (!response.ok) { const payload = await jsonResponse(response); throw new Error(payload?.error || `Quit refused (HTTP ${response.status}).`); }
    if (!jobCurrent(job)) return;
    state.job = null;
    document.querySelector('.toolbar').hidden = true;
    document.querySelector('.workspace').hidden = true;
    document.querySelector('.statusbar').hidden = true;
    $('closed-panel').hidden = false;
    $('closed-panel').focus();
  } catch (error) { finishError(job, error, 'Could not close Studio'); }
}

function emptyDefinition() {
  return { schema: 1, id: '', name: '', metresPerUnit: 1, roundingMetres: 0.01, detail: { vertices: [], triangles: [] }, native: { vertices: [], triangles: [] }, materials: [], faceMaterials: [], engineering: null, flight: { maxSpeed: null }, clearances: [], provenance: {} };
}

function openEditor() {
  $('json-editor').value = JSON.stringify(state.definition ?? emptyDefinition(), null, 2);
  text('editor-error', '');
  $('json-dialog').showModal();
  $('json-editor').focus();
}

function applyEditor() {
  try {
    const definition = editableDefinition(JSON.parse($('json-editor').value));
    $('json-dialog').close();
    setDefinition(definition, 'Definition edited · build required');
  } catch (error) { text('editor-error', error.message); }
}

function selectTab(name) {
  for (const tab of ['inspect', 'help']) {
    const selected = name === tab;
    $(`${tab}-tab`).setAttribute('aria-selected', String(selected));
    $(`${tab}-tab`).tabIndex = selected ? 0 : -1;
    $(`${tab}-panel`).hidden = !selected;
  }
}

for (const [id, key, numeric] of fields) $(id).addEventListener('input', event => editParameter(key, numeric ? inputNumber(event.target.value) : event.target.value));
$('definition-form').addEventListener('submit', event => event.preventDefault());
$('import-button').addEventListener('click', () => $('import-file').click());
$('import-file').addEventListener('change', event => { importFile(event.target.files?.[0]); event.target.value = ''; });
$('save-button').addEventListener('click', saveDefinition);
$('build-button').addEventListener('click', buildDefinition);
$('export-button').addEventListener('click', exportPackage);
$('showroom-button').addEventListener('click', openShowroom);
$('quit-button').addEventListener('click', quitStudio);
for (const key of ['group-source', 'group-target']) $(key).addEventListener('change', updateMergeButton);
$('merge-groups').addEventListener('click', () => {
  const resolved = editableGroups(state.definition);
  const merged = mergeGroups(resolved, Number($('group-source').value), Number($('group-target').value));
  if (merged !== resolved) changeGroups(merged, 'Groups merged · build required');
});
$('auto-groups').addEventListener('click', () => changeGroups(autoGroups(state.definition), 'Auto-grouping selected · build required'));
$('undo-groups').addEventListener('click', () => { if (groupHistory.length) changeGroups(groupHistory.pop(), 'Group edit undone · build required', true); });
$('load-example').addEventListener('click', () => loadExample());
$('edit-json').addEventListener('click', openEditor);
$('apply-edit').addEventListener('click', applyEditor);
for (const id of ['cancel-edit', 'close-dialog']) $(id).addEventListener('click', () => $('json-dialog').close());
for (const mode of ['detail', 'native']) $(`${mode}-mode`).addEventListener('click', () => {
  state.mesh = mode;
  for (const option of ['detail', 'native']) $(`${option}-mode`).setAttribute('aria-pressed', String(option === mode));
  render(false);
});
for (const layer of ['original', 'envelope', 'points', 'clearances']) $(`layer-${layer}`).addEventListener('change', event => viewer.setLayer(layer, event.target.checked));
$('reset-view').addEventListener('click', () => viewer.reset());
for (const tab of ['inspect', 'help']) {
  $(`${tab}-tab`).addEventListener('click', () => selectTab(tab));
  $(`${tab}-tab`).addEventListener('keydown', event => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === 'Home' ? 'inspect' : event.key === 'End' ? 'help' : tab === 'inspect' ? 'help' : 'inspect';
    selectTab(next); $(`${next}-tab`).focus();
  });
}
$('inspect-select').addEventListener('change', () => renderInspect(currentBuild()));

render();
discoverCapabilities();
loadExample(true);
