extern crate vulkano;
extern crate vulkano_win;
extern crate winit;

use winit::{EventsLoop, WindowBuilder};
use vulkano::instance::Instance;

fn main() {

    // Vulkan setup

    let instance = {
        // Ask for a list of Vulkan extensions to draw the window
        let extensions = vulkano_win::required_extensions();

        Instance::new(None, &extensions, None).expect("Could not create Vulkan instance!")
    };

    // Choose physical device to use

    // TODO(Tony): do some filtering to prevent using a device that can't draw to our
    // surface, or a device that doesn't support all the extensions we need

    // TODO(Tony): Check out rust thread stuff, specifically ARC

    let physical = vulkano::instance::PhysicalDevice::enumerate(&instance)
                            .next().expect("No device is available to draw!");

    println!("Using device: {} (type: {:?})", physical.name(), physical.ty());


    // Setup Window
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
