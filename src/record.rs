//! The kernel's B-rep as SolidCraft's other layers take it: the portable record, and the modelling
//! layer's body built from that record. A conversion copies the geometry, and adds to the B-rep
//! the UV trims that the record needs.

use brep_protocol::Refusal;
use solidcore::{Brep, EdgeId, FaceId, Kernel, VertexId};
use solidcore_convert::brep::{Order, to_kernel_body, to_record};
use solidcore_ops::Body;
use solidcore_record::brep::Brep as Record;

pub fn convert_err(e: solidcore_convert::brep::Unsupported) -> Refusal {
    Refusal::failed(format!("the record conversion refused the body: {e}"))
}

pub fn ops_err(e: solidcore_ops::Error) -> Refusal {
    match e {
        solidcore_ops::Error::Argument => Refusal::invalid(e.to_string()),
        _ => Refusal::failed(format!("{e:?}")),
    }
}

/// The record of `b`, with which of its faces, edges and vertices each record index is.
/// The conversion adds the trims the record needs to `b`, so `b` is a build of its own.
pub fn record_of(k: &mut Kernel, b: &mut Brep) -> Result<(Record, Order), Refusal> {
    to_record(k, b).map_err(convert_err)
}

/// The kernel's body built from `record`, as it is.
pub fn brep_of(k: &mut Kernel, record: &Record) -> Result<Brep, Refusal> {
    to_kernel_body(k, record, true).map_err(convert_err)
}

/// `b` as the modelling layer's body. `order` indexes `b`'s entities by record index, and
/// `body_order` indexes the body's: the modelling calls take record indices with the latter.
pub struct Bridged {
    pub body: Body,
    pub order: Order,
    pub body_order: Order,
}

pub fn bridged(k: &mut Kernel, mut b: Brep) -> Result<Bridged, Refusal> {
    let (record, order) = record_of(k, &mut b)?;
    let (body, body_order) = Body::from_record(&record, true).map_err(ops_err)?;
    Ok(Bridged {
        body,
        order,
        body_order,
    })
}

impl Bridged {
    pub fn faces(&self, ids: &[FaceId]) -> Result<Vec<usize>, Refusal> {
        ids.iter().map(|f| index(&self.order.faces, f)).collect()
    }

    pub fn edges(&self, ids: &[EdgeId]) -> Result<Vec<usize>, Refusal> {
        ids.iter().map(|e| index(&self.order.edges, e)).collect()
    }

    pub fn vertices(&self, ids: &[VertexId]) -> Result<Vec<usize>, Refusal> {
        ids.iter().map(|v| index(&self.order.vertices, v)).collect()
    }

    /// The body as the kernel's B-rep again, after the modelling layer changed it.
    pub fn into_brep(self, k: &mut Kernel) -> Result<Brep, Refusal> {
        body_brep(k, &self.body)
    }
}

pub fn body_brep(k: &mut Kernel, body: &Body) -> Result<Brep, Refusal> {
    let (record, _) = body.to_record().map_err(ops_err)?;
    brep_of(k, &record)
}

fn index<T: PartialEq + std::fmt::Debug>(all: &[T], id: &T) -> Result<usize, Refusal> {
    all.iter()
        .position(|x| x == id)
        .ok_or_else(|| Refusal::failed(format!("{id:?} is not in the body's record")))
}
