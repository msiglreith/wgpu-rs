use std::borrow::Cow::Borrowed;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
    platform::desktop::EventLoopExtDesktop,
};
use ndk_glue;

async fn run(mut event_loop: EventLoop<()>, window: Window, swapchain_format: wgpu::TextureFormat) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            // Request an adapter which can render to our surface
            compatible_surface: None,
        })
        .await
        .expect("Failed to find an appropiate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
    let fs_module = device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: Borrowed(&[]),
        push_constant_ranges: Borrowed(&[]),
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: Borrowed("main"),
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: Borrowed("main"),
        }),
        // Use the default rasterizer state: no culling, no depth bias
        rasterization_state: None,
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: Borrowed(&[wgpu::ColorStateDescriptor {
            format: swapchain_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }]),
        depth_stencil_state: None,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: Borrowed(&[]),
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut surface = None;
    let mut swap_chain = None;

    event_loop.run(move |event, _, control_flow| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (
                &instance,
                &adapter,
                &vs_module,
                &fs_module,
                &pipeline_layout,
            );

        *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    println!("----------------------RUST RESIZE----------------------");
                    // Recreate the swap chain with the new size
                    sc_desc.width = size.width;
                    sc_desc.height = size.height;
                    swap_chain = Some(device.create_swap_chain(&surface.as_ref().unwrap(), &sc_desc));
                }
                Event::Resumed =>
                {
                    println!("----------------------RUST RESUME----------------------");
                    surface = Some(unsafe { instance.create_surface(&window) });
                    swap_chain = Some(device.create_swap_chain(&surface.as_ref().unwrap(), &sc_desc));
                    println!("surface: {:?}", surface);
                },
                Event::Suspended =>
                {
                    swap_chain.take();
                    surface.take();
                },
                Event::RedrawRequested(_) => {
                    if let Some(swap_chain) = swap_chain.as_mut() {
                        let frame = swap_chain
                            .get_current_frame()
                            .expect("Failed to acquire next swap chain texture")
                            .output;
                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                        {
                            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                color_attachments: Borrowed(&[wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                        store: true,
                                    },
                                }]),
                                depth_stencil_attachment: None,
                            });
                            rpass.set_pipeline(&render_pipeline);
                            rpass.draw(0..3, 0..1);
                        }

                        queue.submit(Some(encoder.finish()));
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => {}
            }
        });
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
fn do_main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(feature = "subscriber")]
        wgpu::util::initialize_default_subscriber(None);

        // Temporarily avoid srgb formats for the swapchain on the web
        futures::executor::block_on(run(event_loop, window, wgpu::TextureFormat::Rgba8UnormSrgb));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window, wgpu::TextureFormat::Rgba8Unorm));
    }
}

fn main() {
    do_main()
}