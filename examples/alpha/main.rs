#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate cgmath;
extern crate glutin;
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_phase;

mod app;

fn main() {
    let window = glutin::WindowBuilder::new().with_vsync().build().unwrap();
    window.set_title("Alpha: gfx_phase example");
    unsafe { window.make_current() };
    let (w, h) = window.get_inner_size().unwrap();
    let device = gfx_device_gl::create(|s| window.get_proc_address(s));

    let mut app = app::App::new(device, w as u16, h as u16);

    'main: loop {
        // quit when Esc is pressed.
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => break 'main,
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        app.render();

        window.swap_buffers();
        app.graphics.cleanup();
    }
}
