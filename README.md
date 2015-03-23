[![Build Status](https://travis-ci.org/kvark/gfx_scene.png?branch=master)](https://travis-ci.org/kvark/gfx_scene)

## Why high-level rendering?

gfx-rs established a solid basis for API abstraction and safe bind-less draw calls. While is used by simpler apps directly, more complex ones are expected to operate on a higher level. Some elements of this level, like materials, are extremely diverse. Others can be implemented in a rather common way:
  - composing batches from their components
  - batch sorting
  - frustum culling

`gfx_scene` provides a set of abstractions that allow constructing your own rendering systems while having this essential logic implemented automatically. Standard implementations of known rendering pipelines, default scene loaders, and established material models are supposed to follow.

## What is gfx_scene?

High-level rendering and scene management for gfx-rs. It consists of 2 layers.

`gfx_phase` is focused around abstract rendering techniques. Phases implement batch construction and sorting. The user is supposed to:
  - define one or more types of materials
  - define the concept of an entity
  - implement one or more rendering techniques, based on these materials

`gfx_scene` is based on `gfx_phase` and defines the `Entity` type as well as introduces a standard `Scene` struct. In order to get the frustum culling, the user needs to define spatial world that entities live in and provide the bounds. `gfx_scene` is tied to `cgmath-rs` and heavily uses abstract transformations and bounds.

Both layers are very abstract and have a lot of generic parameters. See `alpha` example for the phase usage and `beta` one for the scenes.

## The plan

  1. `gfx-rs`
  	- device abstraction
  	- resource management
  	- bind-less draw calls
  2. `gfx_phase` and `gfx_scene`
    - high-level primitives
    - phases with batch sorting
    - scenes with frustum culling
  3. `gfx_pipeline` and `claymore_*`
    - world implementation
    - forward/deferred/other pipelines
    - PBR/Phong/cartoon materials
    - asset export and loading
