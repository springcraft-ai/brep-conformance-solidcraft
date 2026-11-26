# brep-conformance-solidcraft

The [brep-conformance](https://github.com/springcraft-ai/brep-conformance) adapter for
[SolidCraft](https://github.com/tangency-labs/solidcraft)'s kernel,
[solidcore](https://github.com/tangency-labs/solidcore): it puts solidcore behind the suite's
protocol, so that the suite can score it.

The suite lists it as `solidcraft`, at SolidCraft's release. solidcore is developed inside
SolidCraft's repository, in its `solidcore/` folder, and mirrored to its own repository from there.
The solidcore crate's version tracks the upstream release it was ported from, not the kernel's own
changes, so it stays at 1.0.1 while the kernel moves. A SolidCraft release names one state of the
kernel, and the adapter pins that release's commit: v0.50.0 now.

## Who maintains this

brep-conformance scores a B-rep kernel through an adapter: a small program that answers the suite's
requests with the kernel's own calls. Springcraft, which maintains the suite, wrote this adapter and
maintains it.

- **It is not SolidCraft's.** SolidCraft's maintainers did not write it or choose the tests it runs.
- **A failure can be the adapter's.** A case solidcore fails in the results may be this adapter
  driving it wrongly. Corrections are welcome: open an issue or a pull request.
- **It adds nothing to the kernel.** The adapter is pass-through, as the suite's
  [contract](https://github.com/springcraft-ai/brep-conformance/blob/main/doc/contract.md#what-an-adapter-may-do)
  asks: one call of the kernel for each function, and anything the kernel has no one call for is
  listed as unsupported. What is scored is solidcore, not this code.
- **SolidCraft's maintainers are welcome here.** They know best how to drive solidcore, so their
  review and corrections carry the most weight. Ask in an issue to be added as a reviewer.
- **The suite decides what is scored.** An official run uses only a commit of this adapter that the
  suite's maintainers have reviewed as pass-through, whoever wrote it.

## Overview

[`src/main.rs`](src/main.rs) answers each function with solidcore's one call for it, as
[the contract](https://github.com/springcraft-ai/brep-conformance/blob/main/doc/contract.md#what-an-adapter-may-do)
asks, and lists the rest as unsupported. It builds the suite's box, cylinder, ball, ring and frustum
with solidcore's primitives; its extrusions and revolutions as linear and rotational sweeps of
curves, splines clamped by knot insertion first; its sweeps along the path as one rational quadratic
B-spline; and its lofts as skins, ruled or through every section. Each is built at the origin and
moved onto its frame. It answers the booleans, volume, area, mass properties (summed over the
material regions, so lumps that touch measure too) and validity check with solidcore's own, and
classifies points with its B-rep classifier, which first finds a vertex, edge or face the point lies
on; fillets, one radius or one per edge, and variable fillets with its fillet executive, on the
edges the points name and no others;
offsets and shells with its offset executive; and any similarity with its transform, a mirror as a
scale of −1, and any other affine map with its matrix transform. The bounding box is solidcore's
tight box. A plane section is solidcore's planar section curves, each sampled until every chord's
middle lies within the tolerance, returned as the pieces they are. A boolean of many is solidcore's
merge of a list of B-reps, and a face delete its defeature, the hole closed by extending the
surfaces round it.

Other functions come from SolidCraft's crates beside the kernel, each one call:
[`src/edit.rs`](src/edit.rs) takes chamfers, set-back rounds, face offsets, drafts and splits from
its modelling layer, solidcore-ops; [`src/exchange.rs`](src/exchange.rs) reads STEP with its kernel
boundary's importer, solidcraft-kernel, and writes it with solidcore-step;
[`src/tess.rs`](src/tess.rs) meshes with its mesher, solidcore-tess, on the strict path that refuses
a face it cannot mesh; and [`src/query.rs`](src/query.rs) measures curvature with the modelling
layer's query. A set-back round with no vertex where three of its edges meet is a plain round. A face offset
that the modelling layer refuses because the tangent blends on the moved faces cannot follow them
is unsupported, as its refusal says.
These crates take a body as SolidCraft's portable record, and
[`src/record.rs`](src/record.rs) converts to and from it. The modelling layer runs each call on a
kernel of its own, at solidcore's default tolerances.

This adapter began as
[brep-conformance-solidcore](https://github.com/springcraft-ai/brep-conformance-solidcore), which
pinned the solidcore 1.0.1 crate from crates.io. Two behaviours of solidcore showed in the results
at 1.0.1, as its source read at that release. They have not been checked again since, and the
adapter still answers as it did. An
offset, a shell or a mirror names its new curves under the edit's own identities while the body
still declares the Notify scheme its checker reads, so the checker rejects every such body, though
volume and area are right and each edge meets its vertices. And the zone tolerance is the constant
`1e-5`, which its source names `SM_ZONE_TOL_3D`, not the context's value: every body starts at that
tolerance whatever the kernel options say, and `set_tolerance` only raises entity tolerances. Its
checker also repairs what it checks, so the adapter checks a fresh build and leaves the solid it
measures as it was.

Listed as unsupported, since none of SolidCraft's crates has one call of the protocol's shape for
it. Where SolidCraft reaches the same end another way, the adapter does not assemble it, so that
what is scored is SolidCraft:

- The distance between two solids, the nearest point of a solid, and the interference of a pair:
  its queries measure to a body's boundary, where the protocol's measure to the solid and are zero
  inside it, and none gives a shared volume.
- Every crossing of a ray: its ray query misses where a ray enters some faces with a notch in them,
  and refuses a face on a torus, a spline, a revolution or a blend.
- Outward normals: at a crease its query gives the first face's normal, where the protocol asks
  for none.
- Section areas and Betti numbers: it gives section curves and a count of solids and voids.
- Curvature-continuous rounds (`fillet_g2`, `fillet_setback_g2`): its round takes one size at the
  equal-area thumbweight, and its set-back corner meets such rounds only tangentially.
- Thickening: it thickens a whole sheet body, where the protocol thickens faces of a solid.
- Sewing: it sews bodies, where the protocol sews the faces of a STEP file, which another call
  reads.
- Hidden-line drawings: it draws exact lines and arcs, where the protocol asks for polylines within
  a tolerance.
- A STEP file of several solids: its importer reads each as a body of its own, and joining them into
  one solid set is another call.
- Screws: its screw sweep takes B-spline profile curves only, and lifts each wall within a
  tolerance of its screw surface, where the protocol asks for the exact surface.
- Conic rounds, rounds between faces, full rounds, face moves, slabs and sweeps along a spline: no
  call.

Supported, with behaviour of SolidCraft's that the suite scores:

- A split by a plane that misses the body gives the whole body on both sides, where the protocol
  asks for the body on one side and nothing on the other.
- Curvature at a crease is the nearest face's, where the protocol asks for none; in a hole its sign
  is reversed.
- With its boundary check, the classifier calls a point "on" up to the bodies' 1e-5 mm tolerance
  off a face, though the adapter asks for 1e-7 mm.
- Its record reader refuses a plane whose axes are about 5e-16 off square, as a rotation or the
  modelling layer leaves them, so the calls through the record refuse such bodies.
- Its STEP reader refuses the cones that its writer writes.

A boolean consumes its operands, and `Brep::clone` keeps the original's geometry owners, which
solidcore's checker rejects in the copy and in anything built from it. So the adapter keeps how each
solid was made and rebuilds fresh operands for every boolean. Volume and area are integrated at an
accuracy of 1e-7: at 1e-9 the integrator fails on a ball. The kernel runs at the suite's tolerance
of 1e-7 mm, set through its options, though its bodies keep the 1e-5 mm noted above.

## Setup

The Rust toolchain [`rust-toolchain.toml`](rust-toolchain.toml) pins.

## Usage

The suite runs the adapter as a separate process. Build the adapter here, then run the suite from a
checkout of [brep-conformance](https://github.com/springcraft-ai/brep-conformance) beside this one:

```bash
cargo build --release
cd ../brep-conformance
cargo build --release -p brep-conformance
target/release/brep-conformance --adapter ../brep-conformance-solidcraft/target/release/solidcraft-adapter \
    --out solidcraft.json
```

## Licences

The adapter in this repository is under either the MIT licence
([LICENSE-MIT](LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](LICENSE-APACHE)), at your option.

Either licence covers the adapter and nothing else. It does not cover the kernel, and nothing here
relicenses the kernel.

SolidCraft's crates are separate works under their own licences. In SolidCraft's repository the
solidcore crates declare `Apache-2.0`, and solidcraft-kernel declares `MIT OR Apache-2.0`; that
repository is the authority on that, not this page.

Building the adapter downloads SolidCraft's repository at the pinned commit and links its solidcore
crates and solidcraft-kernel in, so a built adapter contains them and is subject to their terms as
well as this repository's.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
