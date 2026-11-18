//! Exact original-facet geometry. All work is synchronous and explicitly bounded.
//! Source authority and resource authentication belong to the content adapter.
mod adapter;
mod arrangement;
mod budget;
mod certificate;
mod exact;
mod planes;
mod region;
mod section_arrangement;
mod source;

pub use adapter::OriginalAsset;
pub use arrangement::{Arrangement, ArrangementWork, Cell, Segment};
pub use budget::{Budget, BudgetPhase, BudgetRejection, Failure, FailureKind, Limits};
pub use certificate::{ContactStar, FacetWitness, Hull, reconstruct};
pub use exact::{Point2, Point3, Rational, fraction, integer};
pub use source::{AffineTransform, Facet, SourceId, Vertex};

#[cfg(test)]
mod pruning;

#[cfg(test)]
mod section_tests;

#[cfg(test)]
mod adapter_context_tests;
