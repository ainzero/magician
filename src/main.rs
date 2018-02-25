#[macro_use]
extern crate vulkano;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate vulkano_win;
extern crate winit;

use winit::{EventsLoop, WindowBuilder};

use vulkano_win::VkSurfaceBuild;

use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::device::Device;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::Subpass;
use vulkano::instance::Instance;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineBuilder;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain;
use vulkano::swapchain::PresentMode;
use vulkano::swapchain::SurfaceTransform;
use vulkano::swapchain::Swapchain;
use vulkano::swapchain::AcquireError;
use vulkano::swapchain::SwapchainCreationError;
use vulkano::sync::now;
use vulkano::sync::GpuFuture;

use std::fs::File;
use std::io::{BufReader, Read};

use std::sync::Arc;
use std::mem;
use std::iter;

fn read_file(file_name: &str) -> std::io::Result<String> {
    let file = File::open(file_name)?;

    let mut buf_reader = BufReader::new(file);

    let mut contents = String::new();

    buf_reader.read_to_string(&mut contents)?;

    Ok(contents)
}

// TODO(Tony): Finish this implementation
fn initialize_vulkan(event_loop: &EventsLoop) {
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
        .next()
        .expect("No device is available to draw!");

    println!(
        "Using device: {} (type: {:?})",
        physical.name(),
        physical.ty()
    );

    let window = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    window.window().set_title("Magician");

    // Viewport dimensions
    let mut dimensions = {
        let (width, height) = window.window().get_inner_size_pixels().unwrap();
        [width, height]
    };

    // Usually use multiple queues, but the triangle will just use 1
    let queue = physical
        .queue_families()
        .find(|&q| q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false))
        .expect("Couldn't find a graphical queue family");

    // Device initialization
    let (device, mut queues) = {
        let device_ext = vulkano::device::DeviceExtensions {
            khr_swapchain: true,
            ..vulkano::device::DeviceExtensions::none()
        };

        Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue, 0.5)].iter().cloned(),
        ).expect("failed to create device")
    };

    // Get our first and only queue
    let queue = queues.next().unwrap();

    // Initialize Swapchain
    let (mut swapchain, mut images) = {
        let caps = window
            .surface()
            .capabilities(physical)
            .expect("Failed to get surface capabilities");

        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        let format = caps.supported_formats[0].0;
        Swapchain::new(
            device.clone(),
            window.surface().clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            caps.supported_usage_flags,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None,
        ).expect("Failed to create swapchain")
    };

    let vertex_buffer = {
        #[derive(Debug, Clone)]
        struct Vertex {
            position: [f32; 2],
        }
        impl_vertex!(Vertex, position);

        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            [
                Vertex {
                    position: [-0.5, -0.25],
                },
                Vertex {
                    position: [0.0, 0.5],
                },
                Vertex {
                    position: [0.25, -0.1],
                },
            ].iter()
                .cloned(),
        ).expect("failed to create buffer")
    };

    mod vertex_shader {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[path = "./shaders/triangle.vs"]
        struct Dummy;
    }

    mod fragment_shader {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[path = "./shaders/triangle.fs"]
        struct Dummy;
    }

    let vs = vertex_shader::Shader::load(device.clone()).expect("failed to create shader module");
    let fs = fragment_shader::Shader::load(device.clone()).expect("failed to create shader module");

    // Create render pass (which is an object that describes where the output of the graphics
    // pipeline will go)
    let render_pass = Arc::new(
        single_pass_renderpass!(device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap(),
    );

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );


}

fn main() {
    let mut event_loop = EventsLoop::new();
    // initialize_vulkan(&event_loop);

    let window = WindowBuilder::new()
        .with_dimensions(640, 480)
        .with_title("Magician")
        .build(&event_loop)
        .expect("Failed to Create Window!");

    let mut running = true;
    while running {
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
