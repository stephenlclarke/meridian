import { fitScene, rotatePoint, sceneFor } from './geometry.js';

const INITIAL_VIEW = { yaw: -0.63, pitch: -0.31, roll: 0, zoom: 1.24 };
const clamp = (value, low, high) => Math.min(high, Math.max(low, value));
const sub = (a, b) => a.map((value, i) => value - b[i]);
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];

export class HullViewer {
  constructor(canvas, notice) {
    this.canvas = canvas;
    this.notice = notice;
    this.context = canvas.getContext('2d');
    this.view = { ...INITIAL_VIEW };
    this.layers = { original: true, envelope: true, points: false, clearances: true };
    this.scene = sceneFor(null, null);
    this.fit = fitScene(this.scene);
    this.pointers = new Map();
    this.frame = null;
    this.resize = new ResizeObserver(() => this.schedule());
    this.resize.observe(canvas.parentElement);
    this.bindInput();
    this.schedule();
  }

  setScene(definition, report, mode) {
    this.scene = sceneFor(definition, report, mode);
    this.fit = fitScene(this.scene);
    this.notice.textContent = this.scene.notices.join('\n');
    this.notice.hidden = this.scene.notices.length === 0;
    this.schedule();
  }

  setLayer(layer, enabled) {
    this.layers[layer] = enabled;
    this.schedule();
  }

  reset() {
    this.view = { ...INITIAL_VIEW };
    this.schedule();
  }

  zoom(factor) {
    this.view.zoom = clamp(this.view.zoom * factor, 0.08, 20);
    this.schedule();
  }

  bindInput() {
    const canvas = this.canvas;
    canvas.addEventListener('pointerdown', event => {
      if (event.button !== 0 && event.button !== 2) return;
      canvas.focus({ preventScroll: true });
      canvas.setPointerCapture(event.pointerId);
      this.pointers.set(event.pointerId, [event.clientX, event.clientY]);
    });
    canvas.addEventListener('pointermove', event => {
      const old = this.pointers.get(event.pointerId);
      if (!old) return;
      const before = [...this.pointers.values()];
      const next = [event.clientX, event.clientY];
      this.pointers.set(event.pointerId, next);
      if (before.length === 2) {
        const after = [...this.pointers.values()];
        const distance = ([a, b]) => Math.hypot(a[0] - b[0], a[1] - b[1]);
        if (distance(before) > 0) this.zoom(distance(after) / distance(before));
      } else {
        const dx = next[0] - old[0], dy = next[1] - old[1];
        if (event.shiftKey || (event.buttons & 2)) this.view.roll += dx * 0.009;
        else { this.view.yaw += dx * 0.009; this.view.pitch += dy * 0.009; }
        this.schedule();
      }
    });
    for (const name of ['pointerup', 'pointercancel', 'lostpointercapture']) canvas.addEventListener(name, event => this.pointers.delete(event.pointerId));
    canvas.addEventListener('contextmenu', event => event.preventDefault());
    canvas.addEventListener('wheel', event => {
      event.preventDefault();
      const units = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 160 : 1;
      this.zoom(Math.exp(-clamp(event.deltaY * units, -800, 800) * 0.0015));
    }, { passive: false });
    canvas.addEventListener('keydown', event => {
      switch (event.key) {
        case 'ArrowLeft': this.view.yaw -= 0.12; break;
        case 'ArrowRight': this.view.yaw += 0.12; break;
        case 'ArrowUp': this.view.pitch -= 0.12; break;
        case 'ArrowDown': this.view.pitch += 0.12; break;
        case 'q': case 'Q': this.view.roll -= 0.12; break;
        case 'e': case 'E': this.view.roll += 0.12; break;
        case '+': case '=': this.zoom(1.12); break;
        case '-': this.zoom(1 / 1.12); break;
        case 'Home': this.reset(); break;
        default: return;
      }
      event.preventDefault();
      this.schedule();
    });
    canvas.addEventListener('dblclick', () => this.reset());
  }

  schedule() {
    if (this.frame !== null) return;
    this.frame = requestAnimationFrame(() => { this.frame = null; this.draw(); });
  }

  project(point) {
    const rotated = rotatePoint(sub(point, this.fit.center), this.view);
    return [this.width / 2 + rotated[0] * this.scale, this.height * 0.50 - rotated[1] * this.scale, rotated[2] * this.scale];
  }

  line(a, b) {
    this.context.moveTo(a[0], a[1]);
    this.context.lineTo(b[0], b[1]);
  }

  drawGrid() {
    const ctx = this.context;
    const { center, radius, low } = this.fit;
    const floor = low[1] - radius * 0.06;
    const step = 10 ** Math.floor(Math.log10(Math.max(radius / 4, 1e-12)));
    const n = clamp(Math.ceil(radius * 1.7 / step), 4, 18);
    const extent = n * step;
    ctx.lineWidth = 0.75;
    ctx.strokeStyle = '#243747';
    ctx.beginPath();
    for (let i = -n; i <= n; i++) {
      const x = center[0] + i * step, z = center[2] + i * step;
      this.line(this.project([x, floor, center[2] - extent]), this.project([x, floor, center[2] + extent]));
      this.line(this.project([center[0] - extent, floor, z]), this.project([center[0] + extent, floor, z]));
    }
    ctx.stroke();
  }

  drawMesh(mesh, envelope = false) {
    if (!mesh?.faces.length) return;
    const ctx = this.context;
    const projected = mesh.vertices.map(vertex => vertex ? this.project(vertex) : null);
    if (envelope) {
      // One stroke avoids repeated blending at shared or overlapping triangle edges.
      ctx.beginPath();
      for (const face of mesh.faces) {
        const [a, b, c] = face.indices.map(index => projected[index]);
        ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.lineTo(c[0], c[1]); ctx.closePath();
      }
      ctx.strokeStyle = this.layers.original ? '#49dbe064' : '#49dbe0c0';
      ctx.lineWidth = 0.65; ctx.lineJoin = 'round'; ctx.stroke();
      return;
    }
    const sorted = mesh.faces.map(face => {
      const points = face.indices.map(index => projected[index]);
      return { points, color: face.color, depth: points[0][2] + points[1][2] + points[2][2] };
    }).sort((a, b) => a.depth - b.depth);
    ctx.lineJoin = 'round';
    const edges = sorted.length < 5500;
    for (const face of sorted) {
      const [a, b, c] = face.points;
      ctx.beginPath();
      ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.lineTo(c[0], c[1]); ctx.closePath();
      const normal = cross(sub(b, a), sub(c, a));
      const length = Math.hypot(...normal);
      const light = length > 0 ? Math.abs((normal[0] * -0.18 + normal[1] * -0.33 + normal[2] * 0.9) / length) : 0.35;
      const tint = face.color ?? [128, 146, 159];
      const shade = (channel) => Math.round(clamp(channel * (0.45 + 0.55 * light) + 7, 0, 255));
      ctx.fillStyle = `rgb(${tint.map(shade).join(',')})`;
      ctx.fill();
      if (edges) { ctx.strokeStyle = '#142531c0'; ctx.lineWidth = 0.6; ctx.stroke(); }
    }
  }

  drawSupports() {
    const ctx = this.context;
    ctx.fillStyle = '#57e6f1';
    ctx.beginPath();
    for (const point of this.scene.supportPoints) {
      const [x, y] = this.project(point);
      ctx.rect(x - 1.5, y - 1.5, 3, 3);
    }
    ctx.fill();
  }

  drawPaths() {
    const ctx = this.context;
    ctx.lineCap = 'round';
    ctx.font = '11px system-ui, sans-serif';
    for (const path of this.scene.paths) {
      const a = this.project(path.start), b = this.project(path.end);
      const color = path.passed === true ? '#6bda91' : path.passed === false ? '#ff8881' : '#ffb72e';
      if (path.radius > 0) {
        ctx.beginPath(); this.line(a, b); ctx.strokeStyle = `${color}19`;
        ctx.lineWidth = Math.max(1, path.radius * this.scale * 2); ctx.stroke();
      }
      ctx.beginPath(); this.line(a, b); ctx.strokeStyle = color; ctx.lineWidth = 1.7;
      ctx.setLineDash([7, 5]); ctx.stroke(); ctx.setLineDash([]);
      for (const point of [a, b]) {
        ctx.beginPath(); ctx.arc(point[0], point[1], 4, 0, Math.PI * 2);
        ctx.fillStyle = '#10202b'; ctx.fill(); ctx.strokeStyle = color; ctx.stroke();
      }
    }
    ctx.lineCap = 'butt';
  }

  drawGap() {
    if (!this.scene.gap) return;
    const ctx = this.context;
    const [x, y] = this.project(this.scene.gap.point);
    ctx.strokeStyle = '#ffb72e'; ctx.lineWidth = 1.7;
    if (this.scene.gap.nearestPoint) {
      const end = this.project(this.scene.gap.nearestPoint);
      ctx.beginPath(); this.line([x, y], end); ctx.setLineDash([4, 3]); ctx.stroke(); ctx.setLineDash([]);
      ctx.beginPath(); ctx.arc(end[0], end[1], 3, 0, Math.PI * 2); ctx.stroke();
    }
    ctx.beginPath(); ctx.arc(x, y, 6, 0, Math.PI * 2); ctx.stroke();
    ctx.font = '12px system-ui, sans-serif';
    ctx.fillStyle = '#ffce79';
    ctx.fillText(`Largest sampled gap ${this.scene.gap.metres.toFixed(3)} m`, x + 9, y - 9);
  }

  drawAxes() {
    const ctx = this.context;
    const origin = [43, this.height - 58];
    const axes = [[[1, 0, 0], 'X', '#f56c63'], [[0, 1, 0], 'Y', '#9bd35a'], [[0, 0, 1], 'Z', '#69acf4']];
    ctx.font = '13px system-ui, sans-serif';
    for (const [axis, title, color] of axes) {
      const [a, b] = rotatePoint(axis, this.view);
      const end = [origin[0] + a * 35, origin[1] - b * 35];
      ctx.strokeStyle = color; ctx.fillStyle = color; ctx.lineWidth = 2;
      ctx.beginPath(); this.line(origin, end); ctx.stroke();
      ctx.beginPath(); ctx.arc(end[0], end[1], 2.7, 0, Math.PI * 2); ctx.fill();
      ctx.fillText(title, end[0] + (a < 0 ? -13 : 5), end[1] + (b > 0 ? -4 : 14));
    }
  }

  draw() {
    if (!this.context) return;
    const rect = this.canvas.getBoundingClientRect();
    if (rect.width < 2 || rect.height < 2) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    this.width = rect.width; this.height = rect.height;
    const pixelWidth = Math.round(rect.width * dpr), pixelHeight = Math.round(rect.height * dpr);
    if (this.canvas.width !== pixelWidth || this.canvas.height !== pixelHeight) { this.canvas.width = pixelWidth; this.canvas.height = pixelHeight; }
    const ctx = this.context;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = '#0e1924'; ctx.fillRect(0, 0, this.width, this.height);
    this.scale = Math.min(this.width, this.height) * 0.48 / this.fit.radius * this.view.zoom;
    this.drawGrid();
    if (this.layers.original) this.drawMesh(this.scene.source);
    if (this.layers.envelope && this.scene.envelope) this.drawMesh(this.scene.envelope, true);
    if (this.layers.points || (this.layers.envelope && !this.scene.envelope)) this.drawSupports();
    if (this.layers.clearances) this.drawPaths();
    if (this.layers.envelope) this.drawGap();
    this.drawAxes();
  }
}
