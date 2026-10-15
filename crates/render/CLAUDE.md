# The render crate

Rendering traps found the hard way. The root [CLAUDE.md](../../CLAUDE.md)
still applies; this file adds what is particular to drawing with Bevy.

## See-through meshes that overlap must not share a mesh

Two translucent (`AlphaMode::Blend`) layers that can cover each other, such
as a plan or a marker over paint, each need a **mesh asset of their own**,
even an identical copy, or the layer must be opaque.

**Why.** After its distance sort, Bevy merges consecutive see-through items
into one indirect draw when they share a mesh, a pipeline and a material bind
group. Here all three match more often than they seem to:

- `StandardMaterial` is bindless, so every one of them shares a bind group.
- `unlit` is a per-material flag, not part of the pipeline key, so lit and
  unlit materials share a pipeline.

Inside a merged draw, the GPU culling pass gives each instance its slot with
`atomicAdd` (`bevy_pbr`'s `mesh_preprocess.wesl`). Once there are more
instances than one 64-thread workgroup, that order changes every frame, and
so does which layer blends on top. The sort was right all along.

**Symptom.** Over a large area, whole rows of tiles flicker frame to frame
between the top layer's look and the one beneath. A few tiles never show it,
because a single workgroup happens to keep its order.

**What does not fix it:**

- `depth_bias`. Bevy truncates it to an integer for the pipeline key, so
  anything below 1 changes nothing, and the sort it biases was already
  correct.
- `NoIndirectDrawing` on the camera. On this Mac (Metal) nothing 3D drew at
  all, and nothing was logged, so it is no way to make see-through instance
  order deterministic either (I4).

**Where it applies now.** `Palette::hatch_quad`, `hover_quad` and
`select_quad` are copies of `mark_quad` for exactly this reason, and a paint
plan's square is opaque (Q54). A new translucent layer over paint, or over
any other translucent mesh, follows the same rule.

**Reproducing it.** It takes scale: paint most of the screen, put the layer
over all of it, and compare a burst of consecutive frames. A three-tile test
passes either way.
