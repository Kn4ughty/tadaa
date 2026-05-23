use super::Wgpu;

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

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
        event_queue.dispatch_pending(wgpu).unwrap();

        if wgpu.exit || time_of_program_start.elapsed() > Duration::from_secs(8) {
            println!("Exiting..");
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
            conf.step(dt, wgpu.pointer_position, args.mouse_interactive)
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
