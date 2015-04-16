#![feature(plugin, custom_attribute)]
#![plugin(gfx_macros)]

extern crate cgmath;
extern crate glutin;
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_phase;

mod app;

fn main() {
    use gfx::traits::IntoCanvas;

    let mut canvas = gfx_window_glutin::init(glutin::Window::new().unwrap()).into_canvas();
    canvas.output.window.set_title("Alpha: gfx_phase example");
    let aspect = canvas.get_aspect_ratio();
    let mut app = app::App::new(&mut canvas.factory, aspect);

    'main: loop {
        // quit when Esc is pressed.
        for event in canvas.output.window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => break 'main,
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        app.render(&canvas.output, &mut canvas.renderer);

        canvas.present();
    }
}
