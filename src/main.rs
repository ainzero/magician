extern crate winit;

use winit::{EventsLoop,WindowBuilder};

fn main() {
    let mut event_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_dimensions(640,480)
        .with_title("Magician")
        .build(&event_loop)
        .expect("Failed to Create Window!");

    
    let mut running = true;
    while running {
        event_loop.poll_events(|event| {
            if let winit::Event::WindowEvent {event, ..} = event {
                match event {
                    winit::WindowEvent::Closed => running = false,
                    _ => ()
                }
            }
        });
    }




}

