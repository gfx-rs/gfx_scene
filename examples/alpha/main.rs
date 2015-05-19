extern crate cgmath;
extern crate glutin;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_phase;

mod app;

fn main() {
    use gfx::traits::*;

    let window = glutin::WindowBuilder::new()
        .with_title("Alpha: gfx_phase example".to_string())
        .with_vsync()
        .with_gl(glutin::GL_CORE)
        .build().unwrap();
    let (mut stream, mut device, mut factory) = gfx_window_glutin::init(window);

    let aspect = stream.get_aspect_ratio();
    let mut app = app::App::new(&mut factory, aspect);

    'main: loop {
        // quit when Esc is pressed.
        for event in stream.out.window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => break 'main,
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        app.render(&mut stream);

        //stream.present(&mut device);
        stream.flush(&mut device);
        stream.out.window.swap_buffers();
        device.cleanup();
    }
}
