#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate cgmath;
extern crate glutin;
extern crate gfx;
extern crate gfx_device_gl;
extern crate draw_state;
extern crate gfx_phase;

use gfx::traits::*;
use gfx_phase::{QueuePhase, FlushPhase};

#[vertex_format]
#[derive(Copy)]
struct Vertex {
    #[as_float]
    #[name = "a_Pos"]
    pos: [i8; 2],
}

impl Vertex {
    fn new(x: i8, y: i8) -> Vertex {
        Vertex {
            pos: [x, y],
        }
    }
}

#[shader_param]
#[derive(Clone)]
struct Params<R: gfx::Resources> {
    offset: [f32; 2],
    color: [f32; 4],
    _dummy: std::marker::PhantomData<R>,
}

static VERTEX_SRC: &'static [u8] = b"
    #version 120
    attribute vec2 a_Pos;
    uniform vec2 offset;
    void main() {
        vec2 pos = (a_Pos + offset)/10.0;
        gl_Position = vec4(pos, 0.0, 1.0);
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

#[derive(Copy)]
struct SpaceData(cgmath::Vector2<f32>);

impl gfx_phase::ToDepth for SpaceData {
    type Depth = f32;
    fn to_depth(&self) -> f32 {0.0}
}

impl<R: gfx::Resources> gfx_phase::Technique<R, Material, SpaceData>
for Technique<R> {
    type Kernel = ();
    type Params = Params<R>;

    fn test(&self, _: &gfx::Mesh<R>, _: &Material) -> Option<()> {
        Some(())
    }

    fn compile<'a>(&'a self, _: (), _: SpaceData)
                   -> gfx_phase::TechResult<'a, R, Params<R>> {
        (   &self.program,
            Params {
                offset: [0.0; 2],
                color: [0.4, 0.5, 0.6, 0.0],
                _dummy: std::marker::PhantomData,
            },
            None,
            &self.state,
        )
    }

    fn fix_params(&self, _: &Material, space: &SpaceData, params: &mut Params<R>) {
        use cgmath::FixedArray;
        params.offset = *space.0.as_fixed();
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
    let window = glutin::WindowBuilder::new().with_vsync().build_strict().unwrap();
    window.set_title("Beta: gfx_scene example");
    unsafe { window.make_current() };
    let (w, h) = window.get_inner_size().unwrap();
    let frame = gfx::Frame::new(w as u16, h as u16);

    let mut device = gfx_device_gl::GlDevice::new(|s| window.get_proc_address(s));
    let mut renderer = device.create_renderer();
    let mut context = gfx::batch::Context::new();

    let vertex_data = [
        Vertex::new(0, 1),
        Vertex::new(0, 0),
        Vertex::new(1, 1),
        Vertex::new(1, 0),
    ];

    let mesh = device.create_mesh(&vertex_data);
    let slice = mesh.to_slice(gfx::PrimitiveType::TriangleStrip);

    let entities: Vec<_> = (0..10).map(|_| Entity {
        mesh: mesh.clone(),
        slice: slice.clone(),
        material: Material,
    }).collect();

    let mut phase = gfx_phase::Phase::new_cached(
        "Main",
        Technique::new(&mut device),
    );
    phase.sort.push(gfx_phase::Sort::Program);

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
        renderer.clear(clear_data, gfx::COLOR, &frame);

        for (i, ent) in entities.iter().enumerate() {
            use std::num::Float;
            use cgmath::vec2;
            let angle = (i as f32) / (entities.len() as f32) * std::f32::consts::PI * 2.0;
            let offset = vec2(4.0 * angle.cos(), 4.0 * angle.sin());
            let space_data = SpaceData(offset);
            phase.enqueue(ent, space_data, &mut context).unwrap();
        }
        phase.flush(&frame, &mut context, &mut renderer).unwrap();
        
        device.submit(renderer.as_buffer());
        window.swap_buffers();
        device.after_frame();
    }
}
