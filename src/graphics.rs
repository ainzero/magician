pub mod render_manager {
    use vulkano;
    use vulkano_win;

    use vulkano_win::VkSurfaceBuild;
    use vulkano_win::Window;

    use vulkano::instance::Instance;
    use vulkano::buffer::BufferUsage;
    use vulkano::buffer::CpuAccessibleBuffer;
    use vulkano::device::Device;
    use vulkano::command_buffer::AutoCommandBufferBuilder;
    use vulkano::command_buffer::DynamicState;
    use vulkano::framebuffer::Framebuffer;
    use vulkano::framebuffer::Subpass;
    use vulkano::pipeline::GraphicsPipeline;
    use vulkano::pipeline::viewport::Viewport;
    use vulkano::swapchain;
    use vulkano::swapchain::PresentMode;
    use vulkano::swapchain::SurfaceTransform;
    use vulkano::swapchain::Swapchain;
    use vulkano::swapchain::AcquireError;
    use vulkano::swapchain::SwapchainCreationError;
    use vulkano::image::SwapchainImage;
    use vulkano::sync::now;
    use vulkano::sync::GpuFuture;
    use std::sync::Arc;
    use vulkano::device::Queue;

    use vulkano::framebuffer::RenderPassAbstract;

    use vulkano::buffer::BufferAccess;
    use vulkano::pipeline::GraphicsPipelineAbstract;

    use winit::{EventsLoop, WindowBuilder};

    use std::mem;
    use std::marker::Send;
    use std::marker::Sync;
    use std::boxed::Box;
    use std::vec::Vec;
    use std::option::Option;

    #[derive(Debug, Clone)]
    struct Vertex {
        position: [f32; 2],
    }
    impl_vertex!(Vertex, position);

    #[derive(Clone)]
    struct VulkanRenderComponents {
        device: Arc<Device>,
        swapchain: Arc<Swapchain>,
        images: Vec<Arc<SwapchainImage>>,
        framebuffers: Option<
            Vec<Arc<Framebuffer<Arc<RenderPassAbstract + Send + Sync>, ((), Arc<SwapchainImage>)>>>,
        >,
        render_pass: Arc<RenderPassAbstract + Send + Sync>,
        queue: Arc<Queue>,
        vertex_buffer: Vec<Arc<BufferAccess + Send + Sync>>,
        pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,
        dimensions: [u32; 2],
    }

    #[derive(Clone)]
    pub struct RenderManager {
        render_components: Option<VulkanRenderComponents>,
    }

    impl RenderManager {
        pub fn new() -> RenderManager {
            RenderManager {
                render_components: None,
            }
        }

        pub fn startup(&mut self, ref event_loop: &EventsLoop) -> Window {
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
                .with_dimensions(640, 480)
                .with_title("Magician")
                .build_vk_surface(&event_loop, instance.clone())
                .unwrap();

            // Viewport dimensions
            let dimensions = {
                let (width, height) = window.window().get_inner_size_pixels().unwrap();
                [width, height]
            };

            // Usually use multiple queues, but the triangle will just use 1
            let queue = physical
                .queue_families()
                .find(|&q| {
                    q.supports_graphics() && window.surface().is_supported(q).unwrap_or(false)
                })
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
            let (swapchain, images) = {
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

            let vs = vertex_shader::Shader::load(device.clone())
                .expect("failed to create shader module");
            let fs = fragment_shader::Shader::load(device.clone())
                .expect("failed to create shader module");

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

            let gp_builder = GraphicsPipeline::start();

            let gp_temp: vulkano::pipeline::GraphicsPipelineBuilder<
                vulkano::pipeline::vertex::SingleBufferDefinition<Vertex>,
                vulkano::pipeline::shader::EmptyEntryPointDummy,
                (),
                vulkano::pipeline::shader::EmptyEntryPointDummy,
                (),
                vulkano::pipeline::shader::EmptyEntryPointDummy,
                (),
                vulkano::pipeline::shader::EmptyEntryPointDummy,
                (),
                vulkano::pipeline::shader::EmptyEntryPointDummy,
                (),
                (),
            > = gp_builder.vertex_input_single_buffer();

            let pipeline = gp_temp
                .vertex_shader(vs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fs.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap();

            let framebuffers: Option<
                Vec<
                    Arc<
                        Framebuffer<
                            Arc<RenderPassAbstract + Send + Sync>,
                            ((), Arc<SwapchainImage>),
                        >,
                    >,
                >,
            > = None;

            self.render_components = Some(VulkanRenderComponents {
                device: device,
                swapchain: swapchain,
                images: images,
                framebuffers: framebuffers,
                dimensions: dimensions,
                queue: queue,
                render_pass: render_pass,
                pipeline: Arc::new(pipeline),
                vertex_buffer: vec![vertex_buffer],
            });

            window
        }

        pub fn render(self, ref window: &vulkano_win::Window) {
            if self.render_components.is_some() {
                let vulkan_components = self.render_components.unwrap();

                let device = vulkan_components.device;
                let dimensions = vulkan_components.dimensions;
                let mut framebuffers = vulkan_components.framebuffers;
                let mut images = vulkan_components.images;
                let pipeline = vulkan_components.pipeline;
                let queue = vulkan_components.queue;
                let render_pass = vulkan_components.render_pass;
                let mut swapchain = vulkan_components.swapchain;
                let vertex_buffer = vulkan_components.vertex_buffer;

                let mut recreate_swapchain = false;

                let mut previous_frame_end = Box::new(now(device.clone())) as Box<GpuFuture>;

                previous_frame_end.cleanup_finished();

                if recreate_swapchain {
                    // Get the new dimensions for the viewport/framebuffers.
                    let dimensions = {
                        let (new_width, new_height) =
                            window.window().get_inner_size_pixels().unwrap();
                        [new_width, new_height]
                    };

                    let (new_swapchain, new_images) = match swapchain
                        .recreate_with_dimension(dimensions)
                    {
                        Ok(r) => r,
                        // This error tends to happen when the user is manually resizing the window.
                        // Simply restarting the loop is the easiest way to fix this issue.
                        Err(SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(err) => panic!("{:?}", err),
                    };

                    mem::replace(&mut swapchain, new_swapchain);
                    mem::replace(&mut images, new_images);

                    framebuffers = None;

                    recreate_swapchain = false;
                }

                if framebuffers.is_none() {
                    let new_framebuffers = Some(
                        images
                            .iter()
                            .map(|image| {
                                Arc::new(
                                    Framebuffer::start(render_pass.clone())
                                        .add(image.clone())
                                        .unwrap()
                                        .build()
                                        .unwrap(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                    mem::replace(&mut framebuffers, new_framebuffers);
                }

                let (image_num, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(err) => panic!("{:?}", err),
                    };

                let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                    device.clone(),
                    queue.family(),
                ).unwrap()
                    .begin_render_pass(
                        framebuffers.as_ref().unwrap()[image_num].clone(),
                        false,
                        vec![[0.0, 0.0, 1.0, 1.0].into()],
                    )
                    .unwrap()
                    .draw(
                        pipeline.clone(),
                        DynamicState {
                            line_width: None,
                            // TODO: Find a way to do this without having to dynamically allocate a Vec every frame.
                            viewports: Some(vec![
                                Viewport {
                                    origin: [0.0, 0.0],
                                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                                    depth_range: 0.0..1.0,
                                },
                            ]),
                            scissors: None,
                        },
                        vertex_buffer.clone(),
                        (),
                        (),
                    )
                    .unwrap()
                    .end_render_pass()
                    .unwrap()
                    .build()
                    .unwrap();

                let future = previous_frame_end
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush()
                    .unwrap();
                previous_frame_end = Box::new(future) as Box<_>;
            } else {
                // TODO(Z): Fix this to stop program
                println!("Someone didn't start the renderer...")
            }
        }
    }
}
