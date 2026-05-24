use super::{MouseButton, Wgpu};
use crate::Vertex;

use std::time::{Duration, Instant};
use wayland_client::EventQueue;

use crate::confetti::ConfettiPiece;

pub fn main_loop(args: super::Args, wgpu: &mut Wgpu, event_queue: &mut EventQueue<Wgpu>) {
    let conf_count = args.confetti_count as usize;

    let mut confetti = Vec::with_capacity(conf_count);

    for _ in 0..100 {
        // And repeat for other side
        confetti.push(ConfettiPiece::new_random([-1.0, -1.0], 1.0));
        confetti.push(ConfettiPiece::new_random([1.0, -1.0], -1.0));
    }

    let time_of_program_start = Instant::now();

    let mut last_frame_time = Instant::now();
    // Target 60 fps.
    let frame_delay = Duration::from_secs_f32(1.0 / 60.0);

    let mut leafblower = LeafBlower {
        position: [0.0, 0.0],
        angle: 0.0,
    };
    let mut display_leafblower = false;

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.dispatch_pending(wgpu).unwrap();

        if wgpu.exit || time_of_program_start.elapsed() > Duration::from_secs(8) {
            println!("Exiting..");
            break;
        }

        while let Some(pointer_event) = wgpu.pointer_click_queue.pop() {
            use smithay_client_toolkit::seat::pointer::PointerEventKind;

            match pointer_event.kind {
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => {
                    if let Ok(button) = MouseButton::try_from(button)
                        && button == MouseButton::Right
                    {
                        leafblower.position = wgpu.pointer_position;
                        display_leafblower = true;
                    }
                }
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => {
                    if let Ok(button) = MouseButton::try_from(button)
                        && button == MouseButton::Right
                    {
                        leafblower.position = wgpu.pointer_position;
                        display_leafblower = false;
                    }
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let mut dt = now.duration_since(last_frame_time).as_secs_f32();
        // clamp dt to prevent massive jumps
        if dt > 0.1 {
            dt = 0.1
        };
        last_frame_time = now;

        for conf in &mut confetti {
            conf.step(dt, wgpu.pointer_position, args.mouse_interactive)
        }

        if display_leafblower {
            leafblower.step(dt, wgpu.pointer_position);
            for conf in &mut confetti {
                conf.blow(
                    leafblower.position,
                    leafblower.angle + std::f32::consts::PI,
                    0.5,
                    std::f32::consts::PI / 12.0, // 15 deg cone
                );
            }
        }

        if args.indicate_mouse_pos {
            // println!("{:?}", wgpu.pointer_position);
            confetti[0] = ConfettiPiece {
                position: wgpu.pointer_position,
                dimensions: [0.02, 0.02],
                colour: [0.0, 0.0, 0.0],
                velocity: [0.0, 0.0],
                time_alive: 0.1,
                sway_speed: 0.1,
                rotation: 0.1,
                angular_velocity: 0.1,
            };
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

        if display_leafblower {
            let offset: u16 = vertices.len().try_into().unwrap();
            let aspect = wgpu.height as f32 / wgpu.width as f32;
            let (verts, idx) = leafblower.to_quad(aspect);

            for v in verts {
                vertices.push(v);
            }
            for i in idx {
                indices.push(i + offset);
            }
        }

        let vertex_bytes = bytemuck::cast_slice(&vertices);
        let index_bytes = bytemuck::cast_slice(&indices);

        ensure_buffer(
            &wgpu.device,
            &wgpu.queue,
            &mut wgpu.vertex_buffer,
            vertex_bytes,
            wgpu::BufferUsages::VERTEX,
        );
        ensure_buffer(
            &wgpu.device,
            &wgpu.queue,
            &mut wgpu.index_buffer,
            index_bytes,
            wgpu::BufferUsages::INDEX,
        );

        wgpu.num_indices = indices.len() as u32;

        wgpu.render();
        let elapsed = now.elapsed();
        if elapsed < frame_delay {
            std::thread::sleep(frame_delay - elapsed);
        }
    }
}

use wgpu::util::DeviceExt;
fn ensure_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    data: &[u8],
    usage: wgpu::BufferUsages,
) {
    if data.len() > buffer.size() as usize {
        *buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: data,
            usage: usage | wgpu::BufferUsages::COPY_DST,
        });
    } else {
        // Buffer is big enough, just write
        queue.write_buffer(buffer, 0, data);
    }
}

struct LeafBlower {
    position: [f32; 2],
    // Angle in radians
    angle: f32,
}

impl LeafBlower {
    pub fn step(&mut self, _dt: f32, cursor_pos: [f32; 2]) {
        // angle self towards pointer
        let dx = self.position[0] - cursor_pos[0];
        let dy = self.position[1] - cursor_pos[1];

        let angle = f32::atan2(dy, dx);
        self.angle = angle;
    }

    /// Returns a quad of the current leafblower
    pub fn to_quad(&self, aspect: f32) -> ([Vertex; 4], [u16; 6]) {
        let w = 0.2;
        let h = 0.1;

        #[rustfmt::skip]
        let uv_coords = [
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
            [1.0, 1.0], // bottom right
            [0.0, 1.0], // bottom left
        ];

        #[rustfmt::skip]
        let local_verts = [
            [-w,  h], // top left
            [ w,  h], // top right
            [ w, -h], // bottom right
            [-w, -h], // bottom left
        ];
        let cos_r = f32::cos(self.angle);
        let sin_r = f32::sin(self.angle);

        let mut rotated_verts = [Vertex {
            position: [0.0, 0.0],
            colour: [0.0, 0.0, 0.0],
            uv: [-1.0, -1.0],
        }; 4];

        for i in 0..4 {
            let lx = local_verts[i][0];
            let ly = local_verts[i][1];

            // Standard 2d rotation matrix
            // let rx = lx * cos_r - ly * sin_r;
            // let ry = lx * sin_r + ly * cos_r;
            let rx = lx * cos_r - (ly * aspect) * sin_r;
            let ry = (lx * sin_r + (ly * aspect) * cos_r) / aspect;

            rotated_verts[i] = Vertex {
                position: [self.position[0] + rx, self.position[1] + ry],
                colour: [0.0, 0.0, 0.0],
                uv: uv_coords[i],
            }
        }

        let indicies = [0, 3, 1, 1, 3, 2];

        (rotated_verts, indicies)
    }
}
