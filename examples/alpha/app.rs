use std::marker::PhantomData;
use cgmath::{Matrix, Matrix4, Point3, Vector3, vec3};
use cgmath::{FixedArray, Transform, AffineMatrix3};
use gfx;
use gfx::traits::*;
use gfx_phase;
use gfx_phase::{QueuePhase, FlushPhase};

#[vertex_format]
#[derive(Copy)]
struct Vertex {
    #[as_float]
    #[name = "a_Pos"]
    pos: [i8; 3],
}

impl Vertex {
    fn new(x: i8, y: i8, z: i8) -> Vertex {
        Vertex {
            pos: [x, y, z],
        }
    }
}

#[shader_param]
#[derive(Clone)]
struct Params<R: gfx::Resources> {
    transform: [[f32; 4]; 4],
    color: [f32; 4],
    _dummy: PhantomData<R>,
}

static VERTEX_SRC: &'static [u8] = b"
    #version 120
    attribute vec3 a_Pos;
    uniform mat4 transform;
    void main() {
        gl_Position = transform * vec4(a_Pos, 1.0);
    }
";

static FRAGMENT_SRC: &'static [u8] = b"
    #version 120
    uniform vec4 color;
    void main() {
        gl_FragColor = color;
    }
";

// Defining the technique, material, and entity

struct Technique<R: gfx::Resources> {
    program: gfx::ProgramHandle<R>,
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

#[derive(Copy)]
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

    fn compile<'a>(&'a self, kernel: bool, space: ViewInfo)
                   -> gfx_phase::TechResult<'a, R, Params<R>> {
        (   &self.program,
            Params {
                transform: space.0.into_fixed(),
                color: [0.4, 0.5, 0.6, 0.0],
                _dummy: PhantomData,
            },
            None,
            if kernel {&self.state_transparent} else {&self.state_opaque},
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

impl<R: gfx::Resources> gfx_phase::Entity<R, Material> for Entity<R> {
    fn get_material(&self) -> &Material { &self.material }
    fn get_mesh(&self) -> (&gfx::Mesh<R>, &gfx::Slice<R>) { (&self.mesh, &self.slice) }
}

//----------------------------------------

pub struct App<D: gfx::Device> {
    pub device: D,
    frame: gfx::Frame<D::Resources>,
    renderer: gfx::Renderer<D::Resources, D::CommandBuffer>,
    context: gfx::batch::Context<D::Resources>,
    
    phase: gfx_phase::CachedPhase<D::Resources, Material, ViewInfo, Technique<D::Resources>>,
    entities: Vec<Entity<D::Resources>>,
    proj_view: Matrix4<f32>,
}

impl<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    D: gfx::Device<Resources = R, CommandBuffer = C> + Factory<R>
> App<D> {
    pub fn new(mut device: D, w: u16, h: u16) -> App<D> {
        use cgmath::{perspective, deg};
        let renderer = device.create_renderer();

        let vertex_data = [
            Vertex::new(-1, -1, -1),
            Vertex::new(0, 2, -1),
            Vertex::new(2, 0, -1),
            Vertex::new(0, 0, 2),
        ];

        let mesh = device.create_mesh(&vertex_data);

        let index_data: &[u8] = &[
            0, 1, 2,
            0, 3, 1,
            1, 3, 2,
            2, 3, 0,
        ];

        let slice = device
            .create_buffer_index(index_data)
            .to_slice(gfx::PrimitiveType::TriangleList);

        let entities = (0..10).map(|i| Entity {
            mesh: mesh.clone(),
            slice: slice.clone(),
            material: Material { alpha: i as f32 / 10.0 },
        });

        let phase = gfx_phase::Phase::new_cached(
            "Main",
            Technique::new(&mut device),
        );

        let aspect = w as f32 / h as f32;
        let proj = perspective(deg(90.0f32), aspect, 1.0, 10.0);
        let view: AffineMatrix3<f32> = Transform::look_at(
            &Point3::new(1.5f32, -5.0, 3.0),
            &Point3::new(0f32, 0.0, 0.0),
            &Vector3::unit_z(),
        );

        App {
            device: device,
            frame: gfx::Frame::new(w, h),
            renderer: renderer,
            context: gfx::batch::Context::new(),
            phase: phase,
            entities: entities.collect(),
            proj_view: proj.mul_m(&view.mat),
        }
    }
}

impl<D: gfx::Device> App<D> {
    pub fn render(&mut self) {
        let clear_data = gfx::ClearData {
            color: [0.3, 0.3, 0.3, 1.0],
            depth: 1.0,
            stencil: 0,
        };
        self.renderer.reset();
        self.renderer.clear(clear_data, gfx::COLOR | gfx::DEPTH, &self.frame);

        for ent in self.entities.iter() {
            use std::num::Float;
            use std::f32::consts::PI;
            let angle = ent.material.alpha * PI * 2.0;
            let model = Matrix4::from_translation(&vec3(
                3.0 * angle.cos(), 0.0, 3.0 * angle.sin()
            ));
            let view_info = ViewInfo(self.proj_view.mul_m(&model));
            self.phase.enqueue(ent, view_info, &mut self.context).unwrap();
        }
        
        self.phase.queue.sort(gfx_phase::Object::back_to_front);
        self.phase.flush(&self.frame, &mut self.context, &mut self.renderer).unwrap();
        self.device.submit(self.renderer.as_buffer());
    }
}
