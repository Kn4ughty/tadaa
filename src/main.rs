use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
};
use std::ptr::NonNull;
use wayland_client::{
    Connection, Proxy, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_surface},
};

use std::time::{Duration, Instant};

mod confetti;
mod hsv_to_rgb;

use confetti::{ConfettiPiece, Vertex};

const SHADER_SOURCE: &str = include_str!("shader.wgsl");

fn main() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let raw_display_ptr = conn.backend().display_ptr() as *mut std::ffi::c_void;

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    // Initialize xdg_shell handlers so we can select the correct adapter
    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor not available");

    let layer_shell_state =
        smithay_client_toolkit::shell::wlr_layer::LayerShell::bind(&globals, &qh)
            .expect("Layer shell not available");

    // let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("xdg shell not available");
    let output_state = OutputState::new(&globals, &qh);
    let output = output_state
        .outputs()
        .next()
        .expect("No wayland outputs found!");

    let surface = compositor_state.create_surface(&qh);
    let wl_region = smithay_client_toolkit::compositor::Region::new(&compositor_state)
        .expect("Cannot make region");

    surface.set_input_region(Some(wl_region.wl_region()));

    let layer_surface = layer_shell_state.create_layer_surface(
        &qh,
        surface,
        smithay_client_toolkit::shell::wlr_layer::Layer::Overlay,
        None::<String>,
        Some(output).as_ref(),
    );

    use smithay_client_toolkit::shell::wlr_layer::Anchor;
    layer_surface.set_anchor(Anchor::all());
    layer_surface.set_keyboard_interactivity(
        smithay_client_toolkit::shell::wlr_layer::KeyboardInteractivity::None,
    );

    // Create the window for adapter selection
    // let window = xdg_shell_state.create_window(surface, WindowDecorations::ServerDefault, &qh);
    // window.set_title("wgpu wayland window");
    // // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
    // window.set_app_id("io.github.smithay.client-toolkit.WgpuExample");
    // window.set_min_size(Some((256, 256)));

    layer_surface.commit();

    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        backend_options: wgpu::BackendOptions::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: Some(Box::new(WaylandDisplayWrapper {
            display_ptr: raw_display_ptr,
        })),
    });

    // Create the raw window handle for the surface.
    let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
        NonNull::new(conn.backend().display_ptr() as *mut _).unwrap(),
    ));
    let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
        NonNull::new(layer_surface.wl_surface().id().as_ptr() as *mut _).unwrap(),
    ));

    let surface = unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_display_handle),
                raw_window_handle,
            })
            .unwrap()
    };

    // Pick a supported adapter
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .expect("Failed to find suitable adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .expect("Failed to request device");

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
    });
    let cap = surface.get_capabilities(&adapter);
    let surface_format = cap.formats[0];
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::desc()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Essential for overlays!
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, // No culling so we see it from both sides
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        cache: None,
        multiview_mask: None,
    });

    let mut confetti = vec![ConfettiPiece {
        colour: [1.0, 0.0, 0.0],
        dimensions: [0.2, 0.1],
        position: [0.0, 0.0],
        velocity: [0.1, 0.5],
    }];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for conf in &confetti {
        // cannot have more than 2^16 indicies
        let offset: u16 = vertices.len().try_into().expect("Too many indicies!");

        let (verts, idx) = conf.to_quad();
        for v in verts {
            vertices.push(v);
        }

        for i in idx {
            indices.push(i + offset);
        }
    }

    // Create a buffer that is writable from the CPU (COPY_DST)
    use wgpu::util::DeviceExt;
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
    });
    let num_indicies = indices.len() as u32;

    let mut wgpu = Wgpu {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),

        exit: false,
        width: 256,
        height: 256,
        window: layer_surface,
        device,
        surface,
        adapter,
        queue,

        vertices,
        vertex_buffer,

        indices,
        num_indices: num_indicies,
        index_buffer,

        render_pipeline,
    };

    let mut last_frame_time = Instant::now();
    // Target 60 fps.
    let frame_delay = Duration::from_secs_f32(1.0 / 60.0);

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.dispatch_pending(&mut wgpu).unwrap();

        if wgpu.exit {
            println!("exiting example");
            break;
        }

        let now = Instant::now();
        let mut dt = now.duration_since(last_frame_time).as_secs_f32();
        // clamp dt to prevent massive jumps
        if dt > 0.1 {
            dt = 0.1
        };
        last_frame_time = now;

        for conf in &mut confetti {
            conf.step(dt)
        }

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for conf in &confetti {
            let offset: u16 = vertices.len().try_into().unwrap();
            let (verts, idx) = conf.to_quad();

            for v in verts {
                vertices.push(v);
            }
            for i in idx {
                indices.push(i + offset);
            }
        }

        wgpu.queue
            .write_buffer(&wgpu.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        wgpu.queue
            .write_buffer(&wgpu.index_buffer, 0, bytemuck::cast_slice(&indices));
        wgpu.num_indices = indices.len() as u32;

        wgpu.render();
        let elapsed = now.elapsed();
        println!("ft: {:?}", elapsed);
        if elapsed < frame_delay {
            std::thread::sleep(frame_delay - elapsed);
        }
    }

    // On exit we must destroy the surface before the window is destroyed.
    drop(wgpu.surface);
    drop(wgpu.window);
}

#[derive(Debug)]
struct WaylandDisplayWrapper {
    display_ptr: *mut std::ffi::c_void,
}

unsafe impl Send for WaylandDisplayWrapper {}
unsafe impl Sync for WaylandDisplayWrapper {}

impl raw_window_handle::HasDisplayHandle for WaylandDisplayWrapper {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        unsafe {
            let non_null = std::ptr::NonNull::new(self.display_ptr).unwrap();
            let handle = WaylandDisplayHandle::new(non_null);
            Ok(raw_window_handle::DisplayHandle::borrow_raw(
                RawDisplayHandle::Wayland(handle),
            ))
        }
    }
}

struct Wgpu {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,

    exit: bool,
    width: u32,
    height: u32,
    window: LayerSurface,

    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,

    vertices: Vec<Vertex>,
    vertex_buffer: wgpu::Buffer,

    indices: Vec<u16>, // No more than 65,000 verticies. Seems reasonable for now
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    render_pipeline: wgpu::RenderPipeline,
}

impl Wgpu {
    pub fn render(&self) {
        let wgpu::CurrentSurfaceTexture::Success(surface_texture) =
            self.surface.get_current_texture()
        else {
            panic!("need surface texture")
        };

        // .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                multiview_mask: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.0,
                            b: 0.2,
                            a: 0.5,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderpass.set_pipeline(&self.render_pipeline);
            renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            renderpass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Submit the command in the queue to execute
        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl CompositorHandler for Wgpu {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        // Not needed for this example.
    }
}

impl OutputHandler for Wgpu {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for Wgpu {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        todo!()
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (new_width, new_height) = configure.new_size;
        self.width = new_width;
        self.height = new_height;

        let adapter = &self.adapter;
        let surface = &self.surface;
        let queue = &self.queue;

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
        self.num_indices = self.indices.len() as u32;

        let cap = surface.get_capabilities(adapter);
        let present_mode = if cap.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            view_formats: vec![cap.formats[0]],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.width,
            height: self.height,
            desired_maximum_frame_latency: 2,
            present_mode,
        };

        surface.configure(&self.device, &surface_config);
    }
}

impl SeatHandler for Wgpu {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

delegate_registry!(Wgpu);

impl ProvidesRegistryState for Wgpu {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

smithay_client_toolkit::delegate_dispatch2!(Wgpu);
