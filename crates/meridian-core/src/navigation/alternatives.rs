//! Pure original-world alternatives. A route never grants visibility, refuelling or arrival.
use super::{NavigationError, OriginalNavigation, SystemId};
use core::fmt;

const EXPOSURE: &[u8] = include_bytes!("../../../../assets/legacy/g01.route-exposure");
const MAX_VISIBLE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    FewestJumps,
    Safer,
}
impl RouteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::FewestJumps => "Fewest jumps",
            Self::Safer => "Safer (hops + hostile exposure)",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct RouteAlternative {
    pub labels: Vec<RouteKind>,
    pub systems: Vec<SystemId>,
    pub distance_ly: f64,
    pub first_leg_distance_ly: f64,
    pub arrival_hostile_fraction_sum: f64,
    pub assumes_refuelling_for_later_legs: bool,
}
impl RouteAlternative {
    pub fn refuelling_assumption(&self) -> &'static str {
        "Later legs, if any, assume the supplied full-tank reach after refuelling. Station stock, payment and launch are not guaranteed."
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteError {
    Navigation(NavigationError),
    CrossGalaxy,
    ExcessiveScope,
    HiddenSystem(SystemId),
    InvalidExposure,
    InvalidScore,
}
impl From<NavigationError> for RouteError {
    fn from(error: NavigationError) -> Self {
        Self::Navigation(error)
    }
}
impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Navigation(error) => fmt::Display::fmt(error, f),
            Self::CrossGalaxy => f.write_str("Route scope must stay in the current galaxy"),
            Self::ExcessiveScope => f.write_str("At most 256 visible systems can be planned"),
            Self::HiddenSystem(_) => {
                f.write_str("Route endpoint is outside the supplied visible scope")
            }
            Self::InvalidExposure => f.write_str("Invalid immutable original route exposure"),
            Self::InvalidScore => f.write_str("Route scores must be finite and positive"),
        }
    }
}
impl std::error::Error for RouteError {}

impl OriginalNavigation {
    /// Caller supplies actual visibility and current/full-tank reach. No state changes.
    /// Only the existing original G01 catalogue is supported; no cross-galaxy fallback.
    pub fn route_alternatives(
        &self,
        from: SystemId,
        to: SystemId,
        visible: &[SystemId],
        first_reach: f64,
        full_tank_reach: f64,
    ) -> Result<Vec<RouteAlternative>, RouteError> {
        if !first_reach.is_finite()
            || first_reach <= 0.0
            || !full_tank_reach.is_finite()
            || full_tank_reach <= 0.0
        {
            return Err(NavigationError::InvalidReach.into());
        }
        if from.galaxy != to.galaxy {
            return Err(RouteError::CrossGalaxy);
        }
        self.system(from)?;
        self.system(to)?;
        // The original accepts a Set. Duplicate caller IDs do not expand visibility.
        let mut scope = Vec::with_capacity(visible.len().min(MAX_VISIBLE));
        for id in visible {
            if !scope.contains(id) {
                if scope.len() == MAX_VISIBLE {
                    return Err(RouteError::ExcessiveScope);
                }
                scope.push(*id);
            }
        }
        scope.sort_unstable_by_key(|id| (id.galaxy, id.system));
        let source = scope
            .iter()
            .position(|id| *id == from)
            .ok_or(RouteError::HiddenSystem(from))?;
        let target = scope
            .iter()
            .position(|id| *id == to)
            .ok_or(RouteError::HiddenSystem(to))?;
        for id in &scope {
            if id.galaxy != from.galaxy {
                return Err(RouteError::CrossGalaxy);
            }
            self.system(*id)?;
        }
        let exposure = decode_exposure(EXPOSURE)?;
        let distances: Vec<Vec<f64>> = scope
            .iter()
            .map(|a| scope.iter().map(|b| self.distance_ly(*a, *b)).collect())
            .collect::<Result<_, _>>()?;
        if distances
            .iter()
            .flatten()
            .any(|d| !d.is_finite() || *d < 0.0)
        {
            return Err(NavigationError::InvalidDistance.into());
        }
        let mut result: Vec<RouteAlternative> = Vec::new();
        for kind in [RouteKind::FewestJumps, RouteKind::Safer] {
            let weights = distances
                .iter()
                .enumerate()
                .map(|(a, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(b, distance)| {
                            let reach = if a == source {
                                first_reach
                            } else {
                                full_tank_reach
                            };
                            (a != b && *distance <= reach).then_some(match kind {
                                RouteKind::FewestJumps => 1.0,
                                RouteKind::Safer => 1.0 + exposure[usize::from(scope[b].system)],
                            })
                        })
                        .collect()
                })
                .collect::<Vec<Vec<Option<f64>>>>();
            let Some(path) = shortest_path(&weights, source, target)? else {
                continue;
            };
            let systems: Vec<_> = path.iter().map(|index| scope[*index]).collect();
            if let Some(existing) = result.iter_mut().find(|route| route.systems == systems) {
                existing.labels.push(kind);
                continue;
            }
            let distance_ly: f64 = path.windows(2).map(|leg| distances[leg[0]][leg[1]]).sum();
            let risk: f64 = path
                .iter()
                .skip(1)
                .map(|index| exposure[usize::from(scope[*index].system)])
                .sum();
            if !distance_ly.is_finite() || !risk.is_finite() {
                return Err(NavigationError::InvalidDistance.into());
            }
            result.push(RouteAlternative {
                labels: vec![kind],
                systems,
                distance_ly,
                first_leg_distance_ly: path.get(1).map_or(0.0, |next| distances[source][*next]),
                arrival_hostile_fraction_sum: risk,
                assumes_refuelling_for_later_legs: path.len() > 2,
            });
        }
        Ok(result)
    }
}

fn decode_exposure(bytes: &[u8]) -> Result<[f64; MAX_VISIBLE], RouteError> {
    if bytes.len() != 10 + MAX_VISIBLE * 10
        || bytes.get(..10) != Some(b"MRISK001\x00\x01".as_slice())
    {
        return Err(RouteError::InvalidExposure);
    }
    let mut values = [0.0; MAX_VISIBLE];
    for (index, row) in bytes[10..].chunks_exact(10).enumerate() {
        let id = u16::from_le_bytes([row[0], row[1]]);
        let value = f64::from_le_bytes(
            row[2..]
                .try_into()
                .map_err(|_| RouteError::InvalidExposure)?,
        );
        if usize::from(id) != index || !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RouteError::InvalidExposure);
        }
        values[index] = value;
    }
    Ok(values)
}

// Original NavigationRouteSearch's stable lowest-system-ID Dijkstra policy. All
// eligible scores are validated before an early destination exit, including hidden
// expensive/unreachable branches. This scoped kernel accepts at most 256 worlds.
pub(crate) fn shortest_path(
    weights: &[Vec<Option<f64>>],
    source: usize,
    target: usize,
) -> Result<Option<Vec<usize>>, RouteError> {
    let count = weights.len();
    if count == 0
        || count > MAX_VISIBLE
        || source >= count
        || target >= count
        || weights.iter().any(|row| row.len() != count)
        || weights
            .iter()
            .flatten()
            .flatten()
            .any(|score| !score.is_finite() || *score <= 0.0)
    {
        return Err(RouteError::InvalidScore);
    }
    let mut costs = vec![f64::INFINITY; count];
    let mut previous = vec![None; count];
    let mut visited = vec![false; count];
    costs[source] = 0.0;
    for _ in 0..count {
        let next = (0..count)
            .filter(|index| !visited[*index] && costs[*index].is_finite())
            .min_by(|a, b| costs[*a].total_cmp(&costs[*b]).then(a.cmp(b)));
        let Some(node) = next else { break };
        if node == target {
            break;
        }
        visited[node] = true;
        for (next, weight) in weights[node].iter().enumerate() {
            if visited[next] {
                continue;
            }
            let Some(weight) = weight else { continue };
            let candidate = costs[node] + *weight;
            if !candidate.is_finite() {
                return Err(RouteError::InvalidScore);
            }
            if candidate < costs[next] {
                costs[next] = candidate;
                previous[next] = Some(node);
            }
        }
    }
    if !costs[target].is_finite() {
        return Ok(None);
    }
    let mut path = Vec::new();
    let mut node = target;
    for _ in 0..count {
        path.push(node);
        if node == source {
            path.reverse();
            return Ok(Some(path));
        }
        node = previous[node].ok_or(RouteError::InvalidScore)?;
    }
    Err(RouteError::InvalidScore)
}

#[cfg(test)]
mod tests;
