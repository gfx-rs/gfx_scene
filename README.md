[![Build Status](https://travis-ci.org/kvark/gfx_scene.png?branch=master)](https://travis-ci.org/kvark/gfx_scene)

## Why high-level rendering?

gfx-rs established a solid basis for API abstraction and safe bind-less draw calls. While is used by simpler apps directly, more complex ones are expected to operate on a higher level. Some elements of this level, like materials, are extremely diverse. Others can be implemented in a rather common way:
  - composing batches from their components
  - batch sorting
  - frustum culling

`gfx_scene` provides a set of abstractions that allow constructing your own rendering systems while having this essential logic implemented automatically. Standard implementations of known rendering pipelines, default scene loaders, and established material models are supposed to follow.

## What is gfx_scene?

High-level rendering and scene management for gfx-rs. A typical application is supposed to:
  - define one or more types of materials
  - define the concept of an entity
  - implement one or more rendering techniques, based on these materials
  - define spatial relationships
  - define the scene, draw entities using techniques wrapped in phases

The repository contains `gfx_phase` and `gfx_scene` crates for different levels of abstractions.

## The plan

  1. `gfx-rs`
  	- device abstraction
  	- resource management
  	- bind-less draw calls
  2. `gfx_scene`
    - high-level primitives
    - phases
    - scenes with frustum culling
  3. `gfx_pipeline` and `claymore`
    - world implementation
    - forward/deferred/other pipelines
    - PBR/Phong/cartoon materials
    - asset export and loading
