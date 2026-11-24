//! A query of SolidCraft's modelling layer: principal curvatures.

use brep_protocol::{Refusal, model};
use solidcore::{Brep, Kernel};

use crate::record::{ops_err, record_of};

/// The principal curvatures `[k1, k2]` at the point of `b`'s boundary nearest each point.
/// The conversion adds trims to `b`, so `b` is a build of its own.
pub fn curvatures(
    k: &mut Kernel,
    b: &mut Brep,
    points: &[model::Vec3],
) -> Result<Vec<Option<[f64; 2]>>, Refusal> {
    let (record, _) = record_of(k, b)?;
    let found =
        solidcore_ops::continuity::principal_curvatures(&record, points).map_err(ops_err)?;
    Ok(found.into_iter().map(|p| p.map(|p| [p.k1, p.k2])).collect())
}
