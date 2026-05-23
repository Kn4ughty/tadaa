use crate::hsv_to_rgb::hsv_to_rgb;
use rand::prelude::*;

pub struct ConfettiPiece {
    /// Centerpoint
    pub position: [f32; 2],
    /// width and height
    pub dimensions: [f32; 2],
    pub colour: [f32; 3],
    pub velocity: [f32; 2],

    pub time_alive: f32,
    pub sway_speed: f32,

    /// in radians
    pub rotation: f32,
    pub angular_velocity: f32,
}

impl ConfettiPiece {
    pub fn new_random(spawn_pos: [f32; 2], x_mul: f32) -> Self {
        let mut rng = rand::rng();

        let hue = rng.random_range(0.0..360.0);
        let col = hsv_to_rgb(hue, 1.0, 1.0);
        let vx = rng.random_range(0.2..1.5);
        let vy = rng.random_range(0.4..5.5);

        let sway_fac = 0.3;
        let sway_speed = rng.random_range(-sway_fac..sway_fac);

        let rotation = rng.random_range(0.0..3.1415926);
        let angular_velocity = rng.random_range(0.5..2.0);

        ConfettiPiece {
            colour: [
                col.0 as f32 / 255.0,
                col.1 as f32 / 255.0,
                col.2 as f32 / 255.0,
            ],
            velocity: [vx * x_mul, vy],
            position: spawn_pos,
            dimensions: [0.005, 0.01],
            sway_speed,
            time_alive: 0.0,
            rotation,
            angular_velocity,
        }
    }
    pub fn step(&mut self, dt: f32, cursor_pos: [f32; 2], force_field: bool) {
        self.time_alive += dt;

        self.rotation += self.angular_velocity * dt; // Drag this too?

        if force_field {
            let dx = self.position[0] - cursor_pos[0];
            let dy = self.position[1] - cursor_pos[1];
            let dist = f32::sqrt(dx * dx + dy * dy);

            if dist < 0.2 {
                // Force field
                // the force pushing away, is inversely propotional to the distance
                // So if the cursor is close, the force is higher
                // Also prevent black holes with max
                let force = f32::max(0.05 / dist, 0.4);
                let angle = f32::atan2(dy, dx);
                let fx = force * f32::cos(angle);
                let fy = force * f32::sin(angle);
                self.velocity[0] += fx * 0.1;
                self.velocity[1] += fy * 0.1;
            }
        }

        // let drag_coefficnet = 1.5;
        let drag_coefficnet = 1.5;
        self.velocity[0] -= self.velocity[0] * drag_coefficnet * dt;
        self.velocity[1] -= self.velocity[1] * drag_coefficnet * dt;

        // Gravity
        self.velocity[1] -= 2.8 * dt;

        let sway = if self.position[1] > -0.9 {
            f32::sin(self.time_alive * self.sway_speed) * 0.5
        } else {
            // we are touching ground, or very close to
            let ground_drag = 2.0;
            self.velocity[0] -= self.velocity[0] * ground_drag * dt;
            0.0
        };

        self.position[0] += (self.velocity[0] + sway) * dt;
        self.position[1] += self.velocity[1] * dt;

        self.position[1] = self.position[1].max(-1.0 + self.dimensions[1]);
    }

    /// returns RELATIVE indidices. Add the current length of the vertex buffer when using
    pub fn to_quad(&self) -> ([Vertex; 4], [u16; 6]) {
        let w = self.dimensions[0];
        let h = self.dimensions[1];

        #[rustfmt::skip]
        let local_verts = [
            [-w,  h], // top left
            [ w,  h], // top right
            [ w, -h], // bottom right
            [-w, -h], // bottom left
        ];
        let cos_r = f32::cos(self.rotation);
        let sin_r = f32::sin(self.rotation);

        let mut rotated_verts = [Vertex {
            position: [0.0, 0.0],
            colour: self.colour,
        }; 4];

        for i in 0..4 {
            let lx = local_verts[i][0];
            let ly = local_verts[i][1];

            // Standard 2d rotation matrix
            let rx = lx * cos_r - ly * sin_r;
            let ry = lx * sin_r - ly * cos_r;

            rotated_verts[i] = Vertex {
                position: [self.position[0] + rx, self.position[1] + ry],
                colour: self.colour,
            }
        }

        let indicies = [0, 3, 1, 1, 3, 2];

        (rotated_verts, indicies)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub colour: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Attribite 0: position.
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Attribute 1: Colour
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
