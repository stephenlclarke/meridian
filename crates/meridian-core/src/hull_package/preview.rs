//! Platform-independent, presentation-only fit used by package preflight and the desktop vertex encoder.

pub struct PreviewFit {
    dimensions: [f64; 3],
    fitted: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewTriangle {
    pub positions: [[f32; 3]; 3],
    pub normal: [f32; 3],
}

impl PreviewFit {
    /// Input coordinates are already in metres; preserve physical dimensions separately from the uniform fit.
    pub fn new(vertices: &[[f64; 3]]) -> Result<Self, &'static str> {
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for point in vertices {
            if point.iter().any(|n| !n.is_finite()) {
                return Err("Nonfinite authored visual point");
            }
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        let dimensions: [f64; 3] = std::array::from_fn(|axis| high[axis] - low[axis]);
        let extent = dimensions.into_iter().fold(0.0, f64::max);
        if !extent.is_finite()
            || extent <= 0.0
            || dimensions.into_iter().any(|n| !n.is_finite() || n < 0.0)
        {
            return Err("Authored visual has no finite extent");
        }
        let center: [f64; 3] = std::array::from_fn(|axis| low[axis] + dimensions[axis] * 0.5);
        let fitted: Vec<[f64; 3]> = vertices
            .iter()
            .map(|point| std::array::from_fn(|axis| (point[axis] - center[axis]) / extent * 32.0))
            .collect();
        Ok(Self { dimensions, fitted })
    }

    pub fn dimensions(&self) -> [f64; 3] {
        self.dimensions
    }

    pub fn triangle(&self, indices: [usize; 3]) -> Result<PreviewTriangle, &'static str> {
        let [Some(a), Some(b), Some(c)] = indices.map(|index| self.fitted.get(index).copied())
        else {
            return Err("Authored visual face index is invalid");
        };
        let normal64 = normal(a, b, c).ok_or("Authored visual face has no finite normal")?;
        let positions = [a, b, c].map(|p| p.map(|n| n as f32));
        let [a32, b32, c32] = positions.map(|p| p.map(f64::from));
        if normal(a32, b32, c32).is_none() {
            return Err("Authored visual face exceeds renderer coordinate precision");
        }
        let normal = normal64.map(|n| n as f32);
        if !normal
            .iter()
            .chain(positions.iter().flatten())
            .all(|n| n.is_finite())
        {
            return Err("Authored visual face has no finite f32 representation");
        }
        Ok(PreviewTriangle { positions, normal })
    }
}

fn normal(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 3]> {
    let [x, y, z] = std::array::from_fn(|i| b[i] - a[i]);
    let [u, v, w] = std::array::from_fn(|i| c[i] - a[i]);
    let n = [y * w - z * v, z * u - x * w, x * v - y * u];
    let length = n[0].hypot(n[1]).hypot(n[2]);
    (length.is_finite() && length > 0.0).then(|| n.map(|axis| axis / length))
}
