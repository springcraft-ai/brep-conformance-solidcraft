//! Meshes by SolidCraft's mesher, on its strict path: a face it cannot mesh is an error, not a
//! face filled leniently or left out.

use brep_protocol::{Display, DisplayEdge, DisplayFace, Mesh, Refusal};
use solidcore::{Brep, Kernel};
use solidcore_tess::{MeshSpec, TessPlan};

use crate::record::record_of;

fn arr(v: solidcore_record::Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn tess_err(e: solidcore_tess::TessError) -> Refusal {
    Refusal::failed(format!("the mesher refused the body: {e}"))
}

/// The plan of `b`'s faces within `tolerance`, and how many faces it has. The conversion adds
/// trims to `b`, so `b` is a build of its own.
fn planned(k: &mut Kernel, b: &mut Brep, tolerance: f64) -> Result<(TessPlan, usize), Refusal> {
    let (record, _) = record_of(k, b)?;
    // A size of 0 sets no floor under the chord, so the mesher aims at the tolerance asked.
    let (plan, _) =
        solidcore_tess::mesh(&record, &MeshSpec::chord(tolerance), 0.0, None).map_err(tess_err)?;
    Ok((plan, record.faces.len()))
}

fn refused(faces: &[solidcore_tess::RefusedFace]) -> Result<(), Refusal> {
    match faces.first() {
        None => Ok(()),
        Some(f) => Err(Refusal::failed(format!(
            "the mesher left {} faces out, the first {f:?}",
            faces.len()
        ))),
    }
}

pub fn mesh(k: &mut Kernel, b: &mut Brep, tolerance: f64) -> Result<Mesh, Refusal> {
    let (plan, _) = planned(k, b, tolerance)?;
    let (welded, left_out) = plan.welded().map_err(tess_err)?;
    refused(&left_out)?;
    Ok(Mesh::new(
        welded.positions.into_iter().map(arr).collect(),
        welded.triangles,
    ))
}

/// Each face's triangles over its own vertices, with the surface's normal at each, and every
/// edge with the faces it bounds. An edge with one face on a solid is that face's seam.
pub fn display(k: &mut Kernel, b: &mut Brep, tolerance: f64) -> Result<Display, Refusal> {
    let (plan, count) = planned(k, b, tolerance)?;
    let (m, left_out) = plan.evaluate();
    refused(&left_out)?;
    let mut faces = vec![DisplayFace::default(); count];
    let mut renumbered = vec![u32::MAX; m.positions.len()];
    for (t, &f) in m.triangles.iter().zip(&m.tri_face) {
        let face = &mut faces[f as usize];
        let corner = t.map(|i| {
            let i = i as usize;
            if renumbered[i] == u32::MAX {
                renumbered[i] = face.positions.len() as u32;
                face.positions.push(arr(m.positions[i]));
                face.normals.push(arr(m.normals[i]));
            }
            renumbered[i]
        });
        face.triangles.push(corner);
    }
    let faces = faces
        .into_iter()
        .map(|f| DisplayFace::new(f.positions, f.normals, f.triangles))
        .collect();
    let (mut edges, mut seams) = (Vec::new(), Vec::new());
    for (points, on) in m.edges.into_iter().zip(m.edge_faces) {
        let points = points.into_iter().map(arr).collect();
        if on.len() == 1 {
            seams.push(points);
        } else {
            edges.push(DisplayEdge::new(points, on, None, false));
        }
    }
    Ok(Display::new(faces, edges, seams))
}
