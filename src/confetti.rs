pub struct ConfettiPiece {
    /// Centerpoint
    pub position: [f32; 2],
    /// width and height
    pub dimensions: [f32; 2],
    pub colour: [f32; 3],
    pub velocity: [f32; 2],
}

impl ConfettiPiece {
    pub fn step(&mut self, dt: f32) {
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;

        // Gravity
        self.velocity[1] -= 1.9 * dt;

        // todo air resistance.
    }

    /// returns RELATIVE indidices. Add the current length of the vertex buffer when using
    pub fn to_quad(&self) -> ([Vertex; 4], [u16; 6]) {
        // 4 verticies.
        // Then it needs 6 indicies, (two triangles)
        let verts = [
            // Top left
            Vertex {
                position: [
                    self.position[0] - self.dimensions[0],
                    self.position[1] + self.dimensions[1],
                ],
                colour: self.colour,
            },
            // Top right
            Vertex {
                position: [
                    self.position[0] + self.dimensions[0],
                    self.position[1] + self.dimensions[1],
                ],
                colour: self.colour,
            },
            // Bottom right
            Vertex {
                position: [
                    self.position[0] + self.dimensions[0],
                    self.position[1] - self.dimensions[1],
                ],
                colour: self.colour,
            },
            // Bottom left
            Vertex {
                position: [
                    self.position[0] - self.dimensions[0],
                    self.position[1] - self.dimensions[1],
                ],
                colour: self.colour,
            },
        ];

        let indicies = [0, 3, 1, 1, 3, 2];

        (verts, indicies)
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
