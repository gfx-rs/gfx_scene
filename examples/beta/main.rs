extern crate cgmath;
extern crate collision;
extern crate glutin;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_phase;
extern crate gfx_scene;
extern crate hprof;

mod app;

fn main() {
    let window = glutin::WindowBuilder::new()
        .with_title("Beta: gfx_scene example".to_string())
        .with_vsync()
        .with_gl(glutin::GL_CORE)
        .build().unwrap();
    let (mut stream, mut device, mut factory) = gfx_window_glutin::init(window);

    let mut app = app::App::new(&mut factory);

    'main: loop {
        // quit when Esc is pressed.
        for event in stream.out.window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => break 'main,
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        hprof::start_frame();
        let g = hprof::enter("render");
        app.render(&mut stream);
        drop(g);

        let g = hprof::enter("present");
        stream.present(&mut device);
        drop(g);
        hprof::end_frame();

        hprof::profiler().print_timing();

    }
}
