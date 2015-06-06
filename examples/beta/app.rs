use std;
use cgmath;
use gfx::attrib::Floater;
use gfx;
use gfx::traits::*;
use gfx_phase;
use gfx_scene;

static SCALE: f32 = 10.0;

gfx_vertex!( Vertex {
    a_Pos@ pos: [Floater<i8>; 2],
});

impl Vertex {
    fn new(x: i8, y: i8) -> Vertex {
        Vertex {
            pos: [Floater(x), Floater(y)],
        }
    }
}

gfx_parameters!( Params {
    u_Offset@ offset: [f32; 2],
    u_Color@ color: [f32; 4],
    u_Scale@ scale: f32,
});

static VERTEX_SRC: &'static [u8] = b"
    #version 150 core
    in vec2 a_Pos;
    uniform vec2 u_Offset;
    uniform float u_Scale;
    void main() {
        vec2 pos = (a_Pos + u_Offset)/u_Scale;
        gl_Position = vec4(pos, 0.0, 1.0);
    }
";

static FRAGMENT_SRC: &'static [u8] = b"
    #version 150 core
    uniform vec4 u_Color;
    out vec4 o_Color;
    void main() {
        o_Color = u_Color;
    }
";

// Defining the technique, material, and entity

struct Technique<R: gfx::Resources> {
    program: gfx::handle::Program<R>,
    state: gfx::DrawState,
}

impl<R: gfx::Resources> Technique<R> {
    pub fn new<F: Factory<R>>(factory: &mut F) -> Technique<R> {
        let program = factory.link_program(VERTEX_SRC, FRAGMENT_SRC).unwrap();
        Technique {
            program: program,
            state: gfx::DrawState::new(),
        }
    }
}

struct Material;
impl gfx_phase::Material for Material {}

#[derive(Clone, Copy)]
struct ViewInfo(cgmath::Vector2<f32>);

impl gfx_phase::ToDepth for ViewInfo {
    type Depth = f32;
    fn to_depth(&self) -> f32 {0.0}
}

impl<R: gfx::Resources> gfx_phase::Technique<R, Material, ViewInfo>
for Technique<R> {
    type Kernel = ();
    type Params = Params<R>;

    fn test(&self, _: &gfx::Mesh<R>, _: &Material) -> Option<()> {
        Some(())
    }

    fn compile<'a>(&'a self, _: (), _: &ViewInfo)
               -> gfx_phase::TechResult<'a, R, Params<R>>
    {
        (   &self.program,
            Params {
                offset: [0.0; 2],
                color: [0.4, 0.5, 0.6, 0.0],
                scale: SCALE,
                _r: std::marker::PhantomData,
            },
            &self.state,
            None,
        )
    }

    fn fix_params(&self, _: &Material, space: &ViewInfo, params: &mut Params<R>) {
        use cgmath::FixedArray;
        params.offset = *space.0.as_fixed();
    }
}

//----------------------------------------

type Transform<S> = cgmath::Decomposed<S, cgmath::Vector3<S>, cgmath::Quaternion<S>>;

impl gfx_scene::ViewInfo<f32, Transform<f32>> for ViewInfo {
    fn new(_: cgmath::Matrix4<f32>, _: Transform<f32>, model: Transform<f32>) -> ViewInfo {
        ViewInfo(cgmath::Vector2::new(model.disp.x, model.disp.y))
    }
}

struct Camera<S>(cgmath::Ortho<S>);

impl<S: cgmath::BaseFloat> gfx_scene::Node for Camera<S> {
    type Transform = Transform<S>;
    fn get_transform(&self) -> Transform<S> {
        cgmath::Transform::identity()
    }
}

impl<S: cgmath::BaseFloat> gfx_scene::Camera<S> for Camera<S> {
    type Projection = cgmath::Ortho<S>;
    fn get_projection(&self) -> cgmath::Ortho<S> {
        self.0.clone()
    }
}

struct Entity<S, R: gfx::Resources> {
    mesh: gfx::Mesh<R>,
    fragments: Vec<gfx_scene::Fragment<R, Material>>,
    transform: Transform<S>,
    bound: cgmath::Aabb3<S>,
}

impl<S: Clone, R: gfx::Resources> gfx_scene::Node for Entity<S, R> {
    type Transform = Transform<S>;
    fn get_transform(&self) -> Transform<S> {
        self.transform.clone()
    }
}

impl<S: Clone, R: gfx::Resources> gfx_scene::Entity<R, Material> for Entity<S, R> {
    type Bound = cgmath::Aabb3<S>;
    fn get_bound(&self) -> Self::Bound {
        self.bound.clone()
    }
    fn get_mesh(&self) -> &gfx::Mesh<R> {
        &self.mesh
    }
    fn get_fragments(&self) -> &[gfx_scene::Fragment<R, Material>] {
        &self.fragments
    }
}

//----------------------------------------

pub struct App<R: gfx::Resources> {
    phase: gfx_phase::Phase<R, Material, ViewInfo, Technique<R>, ()>,
    scene: Vec<Entity<f32, R>>,
    camera: Camera<f32>,
}

impl<R: gfx::Resources> App<R> {
    pub fn new<F: gfx::Factory<R>>(factory: &mut F) -> App<R> {
        let vertex_data = [
            Vertex::new(0, 1),
            Vertex::new(0, 0),
            Vertex::new(1, 1),
            Vertex::new(1, 0),
        ];

        let mesh = factory.create_mesh(&vertex_data);
        let slice = mesh.to_slice(gfx::PrimitiveType::TriangleStrip);

        let num = 10usize;
        let entities = (0..num).map(|i| {
            use cgmath::{Aabb3, Point3, vec2};
            let angle = (i as f32) / (num as f32) * std::f32::consts::PI * 2.0;
            let offset = vec2(4.0 * angle.cos(), 4.0 * angle.sin());
            Entity {
                mesh: mesh.clone(),
                transform: cgmath::Decomposed {
                    scale: 1.0,
                    rot: cgmath::Quaternion::identity(),
                    disp: cgmath::vec3(offset.x, offset.y, 0.0),
                },
                bound: Aabb3::new(Point3::new(0f32, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
                fragments: vec![
                    gfx_scene::Fragment::new(Material, slice.clone()),
                ],
            }
        }).collect();

        let phase = gfx_phase::Phase::new("Main", Technique::new(factory))
                                     .with_sort(gfx_phase::sort::program);

        let camera = Camera(cgmath::Ortho {
            left: -SCALE, right: SCALE,
            bottom: -SCALE, top: SCALE,
            near: -1f32, far: 1f32,
        });

        App {
            phase: phase,
            scene: entities,
            camera: camera,
        }
    }

    pub fn render<S: gfx::Stream<R>>(&mut self, stream: &mut S) {
        use gfx_scene::AbstractScene;
        let clear_data = gfx::ClearData {
            color: [0.3, 0.3, 0.3, 1.0],
            depth: 1.0,
            stencil: 0,
        };
        stream.clear(clear_data);
        let mut culler = gfx_scene::Frustum::new();
        gfx_scene::Context::new(&mut culler, &self.camera)
                .draw(self.scene.iter(), &mut self.phase, stream)
                .unwrap();
    }
}
