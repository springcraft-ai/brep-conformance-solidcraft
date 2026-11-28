//! STEP in and out: SolidCraft's kernel boundary reads a file, and solidcore's STEP crate writes
//! a body's record.

use brep_protocol::Refusal;
use solidcore::geom::tol::USE_DEFAULT;
use solidcore::topology::brep::p3::new_brep;
use solidcore::{Brep, Kernel};
use solidcore_step::write::product::{ExportBody, Header, bodies};

use crate::record::{brep_of, record_of};

/// The one solid a STEP file holds, or an empty solid for a file of none. SolidCraft reads each
/// solid of a file as its own body, and joining several into one solid set is another call, so
/// a file of several is refused.
pub fn read_step(k: &mut Kernel, text: &str) -> Result<Brep, Refusal> {
    let read = solidcraft_kernel::step_import(text)
        .map_err(|e| Refusal::failed(format!("the STEP import refused the file: {e}")))?;
    if read.bodies.is_empty() {
        return new_brep(k, USE_DEFAULT, USE_DEFAULT).map_err(crate::err);
    }
    let [one] = read.bodies.as_slice() else {
        return Err(Refusal::unsupported(format!(
            "the file holds {} solids; SolidCraft reads each as a body of its own",
            read.bodies.len()
        )));
    };
    let record = solidcraft_kernel::brep_record(&one.body)
        .map_err(|e| Refusal::failed(format!("the imported body has no record: {e}")))?;
    brep_of(k, &record)
}

/// `b` as the text of a STEP file. The conversion adds trims to `b`, so `b` is a build of its own.
pub fn write_step(k: &mut Kernel, b: &mut Brep) -> Result<String, Refusal> {
    if b.face_list().is_empty() {
        return Err(Refusal::unsupported(
            "SolidCraft's STEP writer writes no file of no bodies",
        ));
    }
    let (record, _) = record_of(k, b)?;
    let body = ExportBody {
        name: "solid".into(),
        record: &record,
        color: None,
        faces: Vec::new(),
        layer: String::new(),
    };
    bodies(vec![body], &Header::default())
        .map_err(|e| Refusal::failed(format!("the STEP writer refused the body: {e}")))
}
