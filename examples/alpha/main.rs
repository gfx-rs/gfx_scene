#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate cgmath;
extern crate glutin;
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_phase;

use cgmath::{Matrix, Matrix4, Point3, Vector3, vec3};
use cgmath::{FixedArray, Transform, AffineMatrix3};
use gfx::traits::*;
use gfx_phase::AbstractPhase;

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

// The shader_param attribute makes sure the following struct can be used to
// pass parameters to a shader.
#[shader_param]
#[derive(Clone)]
struct Params<R: gfx::Resources> {
    transform: [[f32; 4]; 4],
    color: [f32; 4],
    _dummy: std::marker::PhantomData<R>,
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
        //let opaque = gfx::DrawState::new().depth(gfx::state::Comparison::LessEqual, true);
        let opaque = gfx::DrawState::new();
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
struct SpaceData(cgmath::Matrix4<f32>);

impl gfx_phase::ToDepth for SpaceData {
    type Depth = f32;
    fn to_depth(&self) -> f32 {0.0}
}

impl<R: gfx::Resources> gfx_phase::Technique<R, Material, SpaceData>
for Technique<R> {
    // Would be nice to have Hash implemented for f32 here...
    type Essense = u8; //alpha, normalized
    type Params = Params<R>;

    fn test(&self, _mesh: &gfx::Mesh<R>, mat: &Material) -> Option<u8> {
        use std::num::Float;
        Some((mat.alpha.max(0.0).min(1.0) * 255.9) as u8)
    }

    fn compile<'a>(&'a self, essense: u8, space: SpaceData)
                   -> gfx_phase::TechResult<'a, R, Params<R>> {
        let alpha = essense as f32 / 255.0;
        (   &self.program,
            Params {
                transform: space.0.into_fixed(),
                color: [0.4, 0.5, 0.6, alpha],
                _dummy: std::marker::PhantomData,
            },
            None,
            if alpha < 1.0 {&self.state_transparent} else {&self.state_opaque},
        )
    }

    fn fix_params(&self, mat: &Material, space: &SpaceData, params: &mut Params<R>) {
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

fn main() {
    let window = glutin::Window::new().unwrap();
    window.set_title("glutin initialization example");
    unsafe { window.make_current() };
    let (w, h) = window.get_inner_size().unwrap();
    let frame = gfx::Frame::new(w as u16, h as u16);

    let mut device = gfx_device_gl::GlDevice::new(|s| window.get_proc_address(s));
    let mut renderer = device.create_renderer();
    let mut context = gfx::batch::Context::new();

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
        .create_buffer_static(index_data)
        .to_slice(gfx::PrimitiveType::TriangleList);

    let entities: Vec<_> = (0..10).map(|i| Entity {
        mesh: mesh.clone(),
        slice: slice.clone(),
        material: Material { alpha: i as f32 / 10.0 },
    }).collect();

    let mut phase = gfx_phase::Phase::new_cached(
        "Main",
        Technique::new(&mut device),
    );
    phase.sort.push(gfx_phase::Sort::DrawState);

    let aspect = w as f32 / h as f32;
    let proj = cgmath::perspective(cgmath::deg(90.0f32), aspect, 1.0, 10.0);
    let view: AffineMatrix3<f32> = Transform::look_at(
        &Point3::new(1.5f32, -5.0, 3.0),
        &Point3::new(0f32, 0.0, 0.0),
        &Vector3::unit_z(),
    );
    let proj_view = proj.mul_m(&view.mat);

    let clear_data = gfx::ClearData {
        color: [0.3, 0.3, 0.3, 1.0],
        depth: 1.0,
        stencil: 0,
    };

    'main: loop {
        // quit when Esc is pressed.
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => break 'main,
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        renderer.reset();
        renderer.clear(clear_data, gfx::COLOR | gfx::DEPTH, &frame);

        // somehow, rust doesn't see the namespace... why?
        let p: &mut gfx_phase::AbstractPhase<gfx_device_gl::GlDevice, _, _> = &mut phase;

        for ent in entities.iter() {
            use std::num::Float;
            let angle = ent.material.alpha * std::f32::consts::PI * 2.0;
            let model = Matrix4::from_translation(&vec3(
                3.0 * angle.cos(), 0.0, 3.0 * angle.sin()
            ));
            let space_data = SpaceData(proj_view.mul_m(&model));
            p.enqueue(ent, space_data, &mut context).unwrap();
        }
        p.flush(&frame, &mut context, &mut renderer).unwrap();
        
        device.submit(renderer.as_buffer());
        window.swap_buffers();
        device.after_frame();
    }
}
