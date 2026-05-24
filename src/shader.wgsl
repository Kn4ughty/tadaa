struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) colour: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) use_texture: f32, // 1.0 textured, 0.0 = flat colour
};

@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.colour = model.color;
    out.uv = model.uv;
    out.use_texture = select(0.0, 1.0, model.uv.x >= 0.0 && model.uv.x <= 1.0
                                    && model.uv.y >= 0.0 && model.uv.y <= 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.use_texture > 0.5 {
        return textureSample(t_diffuse, s_diffuse, in.uv);
    }
    return vec4<f32>(in.colour, 1.0);
}
