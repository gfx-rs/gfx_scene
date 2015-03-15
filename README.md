[![Build Status](https://travis-ci.org/kvark/gfx_scene_meta.png?branch=master)](https://travis-ci.org/kvark/gfx_scene_meta)

## High-level rendering with gfx-rs 

High-level rendering and scene management for gfx-rs. A typical application is supposed to:
  - define one or more types of materials
  - define the concept of an entity
  - implement one or more rendering techniques, based on these materials
  - define spatial relationships
  - define the scene, draw entities using techniques wrapped in phases

The repository contains `gfx_phase` and `gfx_scene` crates for different levels of abstractions.

## The Plan

  1. `gfx-rs`
  	- device abstraction
  	- resource management
  	- bind-less draw calls
  2. `gfx_scene`
    - high-level primitives
    - phases
    - scenes with frustum culling
  3. `gfx_pipeline` and `claymore`
    - forward/deferred/other pipelines
    - world implementation
    - asset export and loading
