use std::marker::PhantomData;
use cgmath::{Matrix, Matrix4, Point3, Vector3, vec3};
use cgmath::{FixedArray, Transform, AffineMatrix3};
use gfx;
use gfx::attrib::Floater;
use gfx::traits::*;
use gfx_phase;


gfx_vertex!( Vertex {
    a_Pos@ pos: [Floater<i8>; 3],
});

impl Vertex {
    fn new(x: i8, y: i8, z: i8) -> Vertex {
        Vertex {
            pos: Floater::cast3([x, y, z]),
        }
    }
}

gfx_parameters!( Params {
    u_Transform@ transform: [[f32; 4]; 4],
    u_Color@ color: [f32; 4],
});

static VERTEX_SRC: &'static [u8] = b"
    #version 150 core
    in vec3 a_Pos;
    uniform mat4 u_Transform;
    void main() {
        gl_Position = u_Transform * vec4(a_Pos, 1.0);
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
    state_opaque: gfx::DrawState,
    state_transparent: gfx::DrawState,
}

impl<R: gfx::Resources> Technique<R> {
    pub fn new<F: Factory<R>>(factory: &mut F) -> Technique<R> {
        let program = factory.link_program(VERTEX_SRC, FRAGMENT_SRC).unwrap();
        let opaque = gfx::DrawState::new().depth(gfx::state::Comparison::LessEqual, true);
        let transparent = opaque.clone().blend(gfx::BlendPreset::Alpha);
        Technique {
            program: program,
            state_opaque: opaque,
            state_transparent: transparent,
        }
    }
}

struct Material {
    alpha: f32,
}

impl gfx_phase::Material for Material {}

#[derive(Clone, Copy)]
struct ViewInfo(Matrix4<f32>);

impl gfx_phase::ToDepth for ViewInfo {
    type Depth = f32;
    fn to_depth(&self) -> f32 {
        self.0[3][2] / self.0[3][3]
    }
}

impl<R: gfx::Resources> gfx_phase::Technique<R, Material, ViewInfo>
for Technique<R> {
    type Kernel = bool; // is transparent
    type Params = Params<R>;

    fn test(&self, _mesh: &gfx::Mesh<R>, mat: &Material) -> Option<bool> {
        Some(mat.alpha < 1.0)
    }

    fn compile<'a>(&'a self, kernel: bool)
               -> gfx_phase::TechResult<'a, R, Params<R>> {
        (   &self.program,
            Params {
                transform: [[0.0; 4]; 4],
                color: [0.4, 0.5, 0.6, 0.0],
                _r: PhantomData,
            },
            if kernel {&self.state_transparent} else {&self.state_opaque},
            None,
        )
    }

    fn fix_params(&self, mat: &Material, space: &ViewInfo, params: &mut Params<R>) {
        params.transform = *space.0.as_fixed();
        params.color[3] = mat.alpha;
    }
}

struct Entity<R: gfx::Resources> {
    mesh: gfx::Mesh<R>,
    slice: gfx::Slice<R>,
    material: Material,
}

//----------------------------------------

pub struct App<R: gfx::Resources> {
    phase: gfx_phase::CachedPhase<R, Material, ViewInfo, Technique<R>>,
    entities: Vec<Entity<R>>,
    proj_view: Matrix4<f32>,
}

impl<R: gfx::Resources> App<R> {
    pub fn new<F: gfx::Factory<R>>(factory: &mut F, aspect: f32) -> App<R> {
        use cgmath::{perspective, deg};

        let vertex_data = [
            Vertex::new(-1, -1, -1),
            Vertex::new(0, 2, -1),
            Vertex::new(2, 0, -1),
            Vertex::new(0, 0, 2),
        ];
        let mesh = factory.create_mesh(&vertex_data);

        let index_data: &[u8] = &[
            0, 1, 2,
            0, 3, 1,
            1, 3, 2,
            2, 3, 0,
        ];
        let slice = index_data.to_slice(factory, gfx::PrimitiveType::TriangleList);

        let entities = (0..10).map(|i| Entity {
            mesh: mesh.clone(),
            slice: slice.clone(),
            material: Material { alpha: i as f32 / 10.0 },
        });

        let phase = gfx_phase::Phase::new("Main", Technique::new(factory))
                                     .with_sort(gfx_phase::sort::back_to_front)
                                     .with_cache();

        let proj = perspective(deg(90.0f32), aspect, 1.0, 10.0);
        let view: AffineMatrix3<f32> = Transform::look_at(
            &Point3::new(1.5f32, -5.0, 3.0),
            &Point3::new(0f32, 0.0, 0.0),
            &Vector3::unit_z(),
        );

        App {
            phase: phase,
            entities: entities.collect(),
            proj_view: proj.mul_m(&view.mat),
        }
    }

    pub fn render<S: gfx::Stream<R>>(&mut self, stream: &mut S) {
        use gfx_phase::AbstractPhase;
        let clear_data = gfx::ClearData {
            color: [0.3, 0.3, 0.3, 1.0],
            depth: 1.0,
            stencil: 0,
        };
        stream.clear(clear_data);

        for ent in self.entities.iter() {
            use std::f32::consts::PI;
            let angle = ent.material.alpha * PI * 2.0;
            let model = Matrix4::from_translation(&vec3(
                3.0 * angle.cos(), 0.0, 3.0 * angle.sin()
            ));
            let view_info = ViewInfo(self.proj_view.mul_m(&model));
            self.phase.enqueue(&ent.mesh, &ent.slice, &ent.material, &view_info).unwrap();
        }
        
        self.phase.flush(stream).unwrap();
    }
}
