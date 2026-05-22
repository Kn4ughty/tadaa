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
    pub fn step(&mut self, dt: f32) {
        self.time_alive += dt;

        self.rotation += self.angular_velocity * dt; // Drag this too?

        let drag_coefficnet = 1.5;
        self.velocity[0] -= self.velocity[0] * drag_coefficnet * dt;
        self.velocity[1] -= self.velocity[1] * drag_coefficnet * dt;

        // Gravity
        self.velocity[1] -= 2.8 * dt;

        let sway = f32::sin(self.time_alive * self.sway_speed) * 0.5;

        self.position[0] += (self.velocity[0] + sway) * dt;
        self.position[1] += self.velocity[1] * dt;
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
