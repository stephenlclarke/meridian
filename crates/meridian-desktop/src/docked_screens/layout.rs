use super::{Catalogue, WorldRecord};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }
    pub fn contains(self, p: [f64; 2]) -> bool {
        p.into_iter().all(f64::is_finite)
            && p[0] >= self.x
            && p[0] < self.x + self.width
            && p[1] >= self.y
            && p[1] < self.y + self.height
    }
    pub fn centre(self) -> [f64; 2] {
        [self.x + self.width / 2.0, self.y + self.height / 2.0]
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
}
impl Viewport {
    pub fn new(width: f64, height: f64) -> Option<Self> {
        (width.is_finite() && height.is_finite() && width >= 320.0 && height >= 240.0)
            .then_some(Self { width, height })
    }
    pub fn canvas(self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }
    pub fn body(self) -> Rect {
        Rect::new(16.0, 102.0, self.width - 32.0, self.height - 146.0)
    }
    pub(super) fn panes(self) -> (Rect, Rect) {
        let b = self.body();
        let left = if self.width >= 1000.0 {
            300.0
        } else {
            (b.width * 0.46).min(300.0)
        };
        (
            Rect::new(b.x, b.y, left, b.height),
            Rect::new(b.x + left + 16.0, b.y, b.width - left - 16.0, b.height),
        )
    }
    pub(super) fn rows(self, area: Rect, top: f64) -> usize {
        ((area.height - top).max(0.0) / 20.0).floor() as usize
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartLayout {
    pub plot: Rect,
    pub details: Rect,
    scale: f64,
    centre: [f64; 2],
}
impl ChartLayout {
    /// Source Cartesian light-years; local extent is the source fuel-chart convention.
    pub fn new(
        view: Viewport,
        catalogue: &Catalogue,
        galaxy: u8,
        local: Option<(&WorldRecord, u16)>,
    ) -> Self {
        let body = view.body();
        let (plot, details) = if view.width >= 1000.0 {
            (
                Rect::new(body.x, body.y, body.width - 300.0, body.height),
                Rect::new(body.x + body.width - 284.0, body.y, 284.0, body.height),
            )
        } else {
            (
                Rect::new(body.x, body.y, body.width, body.height * 0.54),
                Rect::new(
                    body.x,
                    body.y + body.height * 0.54 + 8.0,
                    body.width,
                    (body.height * 0.46 - 8.0).max(0.0),
                ),
            )
        };
        let (centre, extent) = if let Some((current, fuel)) = local {
            (
                current.position,
                [f64::from(fuel).mul_add(0.22, 0.0).max(14.0); 2],
            )
        } else {
            let mut min = [f64::INFINITY; 2];
            let mut max = [f64::NEG_INFINITY; 2];
            for w in catalogue.worlds.iter().filter(|w| w.id.galaxy == galaxy) {
                for i in 0..2 {
                    min[i] = min[i].min(w.position[i]);
                    max[i] = max[i].max(w.position[i]);
                }
            }
            if min.into_iter().chain(max).any(|v| !v.is_finite()) {
                ([0.0; 2], [1.0; 2])
            } else {
                (
                    [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0],
                    [(max[0] - min[0]).max(1.0), (max[1] - min[1]).max(1.0)],
                )
            }
        };
        let scale = ((plot.width - 24.0).max(0.0) / extent[0])
            .min((plot.height - 24.0).max(0.0) / extent[1]);
        Self {
            plot,
            details,
            scale,
            centre,
        }
    }
    pub fn point(self, world: &WorldRecord) -> [f64; 2] {
        let p = self.plot.centre();
        [
            p[0] + (world.position[0] - self.centre[0]) * self.scale,
            p[1] + (world.position[1] - self.centre[1]) * self.scale,
        ]
    }
    pub fn pixels_for_ly(self, range: f64) -> Option<f64> {
        (range.is_finite() && range >= 0.0).then_some(range * self.scale)
    }
}
