#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate cgmath;
extern crate glfw;
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_phase;
extern crate gfx_scene;

use cgmath::FixedArray;
use cgmath::{Matrix, Point3, Vector3};
use cgmath::{Transform, AffineMatrix3};
use gfx::{Device, DeviceExt, ToSlice};
use gfx::batch::RefBatch;
use glfw::Context;

#[vertex_format]
#[derive(Copy)]
struct Vertex {
    #[as_float]
    #[name = "a_Pos"]
    pos: [i8; 3],

    #[as_float]
    #[name = "a_TexCoord"]
    tex_coord: [u8; 2],
}

// The shader_param attribute makes sure the following struct can be used to
// pass parameters to a shader.
#[shader_param]
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

//----------------------------------------

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let (mut window, events) = glfw
        .create_window(640, 480, "Cubes example", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.make_current();
    glfw.set_error_callback(glfw::FAIL_ON_ERRORS);
    window.set_key_polling(true);

    let (w, h) = window.get_framebuffer_size();
    let frame = gfx::Frame::new(w as u16, h as u16);

    let mut device = gfx_device_gl::GlDevice::new(|s| window.get_proc_address(s));
    let mut renderer = device.create_renderer();
    let mut context = gfx::batch::Context::new();

    let vertex_data = [
        // top (0, 0, 1)
        Vertex { pos: [-1, -1,  1], tex_coord: [0, 0] },
        Vertex { pos: [ 1, -1,  1], tex_coord: [1, 0] },
        Vertex { pos: [ 1,  1,  1], tex_coord: [1, 1] },
        Vertex { pos: [-1,  1,  1], tex_coord: [0, 1] },
        // bottom (0, 0, -1)
        Vertex { pos: [-1,  1, -1], tex_coord: [1, 0] },
        Vertex { pos: [ 1,  1, -1], tex_coord: [0, 0] },
        Vertex { pos: [ 1, -1, -1], tex_coord: [0, 1] },
        Vertex { pos: [-1, -1, -1], tex_coord: [1, 1] },
        // right (1, 0, 0)
        Vertex { pos: [ 1, -1, -1], tex_coord: [0, 0] },
        Vertex { pos: [ 1,  1, -1], tex_coord: [1, 0] },
        Vertex { pos: [ 1,  1,  1], tex_coord: [1, 1] },
        Vertex { pos: [ 1, -1,  1], tex_coord: [0, 1] },
        // left (-1, 0, 0)
        Vertex { pos: [-1, -1,  1], tex_coord: [1, 0] },
        Vertex { pos: [-1,  1,  1], tex_coord: [0, 0] },
        Vertex { pos: [-1,  1, -1], tex_coord: [0, 1] },
        Vertex { pos: [-1, -1, -1], tex_coord: [1, 1] },
        // front (0, 1, 0)
        Vertex { pos: [ 1,  1, -1], tex_coord: [1, 0] },
        Vertex { pos: [-1,  1, -1], tex_coord: [0, 0] },
        Vertex { pos: [-1,  1,  1], tex_coord: [0, 1] },
        Vertex { pos: [ 1,  1,  1], tex_coord: [1, 1] },
        // back (0, -1, 0)
        Vertex { pos: [ 1, -1,  1], tex_coord: [0, 0] },
        Vertex { pos: [-1, -1,  1], tex_coord: [1, 0] },
        Vertex { pos: [-1, -1, -1], tex_coord: [1, 1] },
        Vertex { pos: [ 1, -1, -1], tex_coord: [0, 1] },
    ];

    let mesh = device.create_mesh(&vertex_data);

    let index_data: &[u8] = &[
         0,  1,  2,  2,  3,  0, // top
         4,  5,  6,  6,  7,  4, // bottom
         8,  9, 10, 10, 11,  8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];

    let slice = device
        .create_buffer_static::<u8>(index_data)
        .to_slice(gfx::PrimitiveType::TriangleList);

    let program = device.link_program(VERTEX_SRC, FRAGMENT_SRC).unwrap();

    let state = gfx::DrawState::new().depth(gfx::state::Comparison::LessEqual, true);

    let batch: RefBatch<Params<_>> =
        context.make_batch(&program, &mesh, slice, &state).unwrap();

    let view: AffineMatrix3<f32> = Transform::look_at(
        &Point3::new(1.5f32, -5.0, 3.0),
        &Point3::new(0f32, 0.0, 0.0),
        &Vector3::unit_z(),
    );
    let aspect = w as f32 / h as f32;
    let proj = cgmath::perspective(cgmath::deg(45.0f32), aspect, 1.0, 10.0);

    let data = Params {
        transform: proj.mul_m(&view.mat).into_fixed(),
        color: [1.0; 4],
        _dummy: std::marker::PhantomData,
    };

    let clear_data = gfx::ClearData {
        color: [0.3, 0.3, 0.3, 1.0],
        depth: 1.0,
        stencil: 0,
    };

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) =>
                    window.set_should_close(true),
                _ => {},
            }
        }

        renderer.clear(clear_data, gfx::COLOR | gfx::DEPTH, &frame);
        renderer.draw(&(&batch, &data, &context), &frame).unwrap();
        device.submit(renderer.as_buffer());
        renderer.reset();

        window.swap_buffers();
    }
}
