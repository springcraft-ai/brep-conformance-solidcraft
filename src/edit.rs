//! The edits SolidCraft makes in one call each, beside the ones `main.rs` drives: from solidcore
//! itself (any affine map, a boolean of many, a face delete) and from its modelling layer
//! (chamfers, set-back rounds, face offsets, drafts and splits).

use brep_protocol::{Op, Refusal, model};
use solidcore::blend::feature_executive::{RebuildBrepType, defeature_and_rebuild};
use solidcore::boolean::api_brep::merge_breps;
use solidcore::geom::a_object::BooleanOperation;
use solidcore::topology::brep::mirror::transform_matrix;
use solidcore::{Brep, Kernel, VertexId};
use solidcore_ops::split::{Tool, split_body};
use solidcore_ops::{Blend, BlendOptions, ChamferSecond, Corners, SetBack};
use solidcore_record::brep::Surface;

use crate::record::{Bridged, body_brep, bridged, ops_err};
use crate::{edges_at, err, faces_at};

/// `b` mapped by `p ↦ M·p + t`, the rows of `m` being `[M | t]`. solidcore's matrix maps row
/// vectors, `p·A + t` with `t` its fourth row, so `A` is `M` transposed.
pub fn affine(k: &mut Kernel, b: &mut Brep, m: &[[f64; 4]; 3]) -> Result<(), Refusal> {
    let mut a = [[0.0; 4]; 4];
    for (i, row) in a.iter_mut().take(3).enumerate() {
        for (j, x) in row.iter_mut().take(3).enumerate() {
            *x = m[j][i];
        }
    }
    a[3] = [m[0][3], m[1][3], m[2][3], 1.0];
    transform_matrix(k, b, &a, true).map_err(err)
}

/// The boolean `op` of every B-rep in one call. solidcore folds the list from its last B-rep to
/// its first, so a difference's target goes last.
pub fn merged(k: &mut Kernel, mut breps: Vec<Brep>, op: Op) -> Result<Brep, Refusal> {
    let operation = match op {
        Op::Union => BooleanOperation::Union,
        Op::Intersection => BooleanOperation::Intersection,
        Op::Difference => BooleanOperation::Difference,
    };
    merge_breps(k, &mut breps, operation).map_err(err)
}

/// `b` with the faces the points lie on taken out as one feature, the hole closed by extending
/// the surfaces round it.
pub fn delete_faces(k: &mut Kernel, b: &mut Brep, at: &[model::Vec3]) -> Result<(), Refusal> {
    let faces = faces_at(k, b, at)?;
    let rebuild = RebuildBrepType::ExtendSurface;
    defeature_and_rebuild(k, b, &faces, rebuild, RebuildBrepType::None).map_err(err)?;
    Ok(())
}

/// The edges the points lie on bevelled, each back `sizes[i]` along both faces, in one blend.
pub fn chamfer(
    k: &mut Kernel,
    b: Brep,
    at: &[model::Vec3],
    sizes: &[f64],
) -> Result<Brep, Refusal> {
    let edges = edges_at(k, &b, at)?;
    let br = bridged(k, b)?;
    let edges = br.edges(&edges)?;
    blend(k, br, Blend::Chamfer, &edges, sizes, &[])
}

/// The edge `edge` lies on bevelled `d1` back along the face `face` lies on and `d2` along the
/// other.
pub fn chamfer_two(
    k: &mut Kernel,
    mut b: Brep,
    edge: model::Vec3,
    face: model::Vec3,
    d1: f64,
    d2: f64,
) -> Result<Brep, Refusal> {
    let e = edges_at(k, &b, &[edge])?;
    let f = faces_at(k, &mut b, &[face])?;
    let br = bridged(k, b)?;
    let (e, f) = (br.edges(&e)?, br.faces(&f)?);
    let second = ChamferSecond::Distance(d2);
    solidcore_ops::chamfer_sides_ordered(&br.body, &br.body_order, &e, &f, d1, second)
        .map_err(ops_err)?;
    br.into_brep(k)
}

/// The edges the points lie on rounded by `radius`, the rounds stopping `setback` short of each
/// vertex where three or more of them meet, its corner filled by the kernel.
pub fn fillet_setback(
    k: &mut Kernel,
    b: Brep,
    at: &[model::Vec3],
    radius: f64,
    setback: f64,
) -> Result<Brep, Refusal> {
    let edges = edges_at(k, &b, at)?;
    let mut ends: Vec<VertexId> = Vec::new();
    for &e in &edges {
        ends.extend(solidcore::query::edge::edge_vertices(&b, e).map_err(err)?);
    }
    let mut corners: Vec<VertexId> = Vec::new();
    for v in &ends {
        if !corners.contains(v) && ends.iter().filter(|w| *w == v).count() >= 3 {
            corners.push(*v);
        }
    }
    if corners.is_empty() {
        return Err(Refusal::invalid("no vertex where three of the edges meet"));
    }
    let br = bridged(k, b)?;
    let edges = br.edges(&edges)?;
    let setbacks: Vec<SetBack> = br
        .vertices(&corners)?
        .into_iter()
        .map(|vertex| SetBack {
            vertex,
            distance: setback,
        })
        .collect();
    blend(k, br, Blend::Round, &edges, &[radius], &setbacks)
}

fn blend(
    k: &mut Kernel,
    br: Bridged,
    kind: Blend,
    edges: &[usize],
    sizes: &[f64],
    setbacks: &[SetBack],
) -> Result<Brep, Refusal> {
    let options = BlendOptions::default();
    let (body, order) = (&br.body, &br.body_order);
    solidcore_ops::blend::blend_ordered(
        body,
        order,
        kind,
        edges,
        sizes,
        setbacks,
        Corners::Filled,
        options,
    )
    .map_err(ops_err)?;
    br.into_brep(k)
}

/// The faces the points lie on moved `distance` along their outward normals, the faces round
/// them extended or trimmed to meet them.
pub fn offset_faces(
    k: &mut Kernel,
    mut b: Brep,
    at: &[model::Vec3],
    distance: f64,
) -> Result<Brep, Refusal> {
    let faces = faces_at(k, &mut b, at)?;
    let br = bridged(k, b)?;
    let faces = br.faces(&faces)?;
    solidcore_ops::offset_faces(&br.body, &br.body_order, &faces, distance).map_err(ops_err)?;
    br.into_brep(k)
}

/// The faces the points lie on tilted by `angle` about where they meet the plane through
/// `origin` square to `pull`.
pub fn draft(
    k: &mut Kernel,
    mut b: Brep,
    at: &[model::Vec3],
    origin: model::Vec3,
    pull: model::Vec3,
    angle: f64,
) -> Result<Brep, Refusal> {
    let faces = faces_at(k, &mut b, at)?;
    let br = bridged(k, b)?;
    let faces = br.faces(&faces)?;
    let (body, order) = (&br.body, &br.body_order);
    solidcore_ops::draft(body, order, &faces, origin, pull, pull, angle).map_err(ops_err)?;
    br.into_brep(k)
}

/// The part of `b` on the side of the plane through `origin` square to `normal` the normal
/// points to (`front`), or the part on the other side. The plane's tool covers `reach` mm about
/// `origin`, as far as the body extends.
pub fn split(
    k: &mut Kernel,
    b: Brep,
    origin: model::Vec3,
    normal: model::Vec3,
    reach: f64,
    front: bool,
) -> Result<Brep, Refusal> {
    let n = solidcore::Vec3::new(normal[0], normal[1], normal[2]);
    let n = n * (1.0 / n.length());
    let hint = if n.x.abs() < 0.9 {
        solidcore::Vec3::new(1.0, 0.0, 0.0)
    } else {
        solidcore::Vec3::new(0.0, 1.0, 0.0)
    };
    let u = hint.cross(n);
    let u = u * (1.0 / u.length());
    let v = n.cross(u);
    let tool = Tool {
        surface: Surface::Plane {
            origin,
            u_axis: [u.x, u.y, u.z],
            v_axis: [v.x, v.y, v.z],
        },
        domain: [[-reach, reach], [-reach, reach]],
    };
    let br = bridged(k, b)?;
    let Some((behind, ahead)) = split_body(&br.body, &tool).map_err(ops_err)? else {
        return Err(Refusal::failed(
            "the split leaves a side empty, or the plane does not cut the body through",
        ));
    };
    body_brep(k, if front { &ahead } else { &behind })
}
