#[macro_use]
extern crate vulkano;
 #[macro_use]
extern crate vulkano_shader_derive;
extern crate vulkano_win;
extern crate winit;

mod graphics;

use graphics::render_manager::RenderManager;

use winit::EventsLoop;

use std::fs::File;
use std::io::{BufReader, Read, Result};

fn read_file(file_name: &str) -> Result<String> {
    let file = File::open(file_name)?;

    let mut buf_reader = BufReader::new(file);

    let mut contents = String::new();

    buf_reader.read_to_string(&mut contents)?;

    Ok(contents)
}

fn main() {
    let mut event_loop = EventsLoop::new();
    
    let mut render_manager = RenderManager::new();

    let window = render_manager.startup(&event_loop);

    let mut running = true;

    while running {
        render_manager.render(&window);
        event_loop.poll_events(|event| {
            if let winit::Event::WindowEvent { event, .. } = event {
                match event {
                    winit::WindowEvent::Closed => running = false,
                    winit::WindowEvent::KeyboardInput { input, .. } => {
                        if input.virtual_keycode.is_some() {
                            let key = input.virtual_keycode.unwrap();

                            if key == winit::VirtualKeyCode::Escape {
                                running = false
                            }
                        }
                    }
                    _ => (),
                }
            }
        });
    } 
}
