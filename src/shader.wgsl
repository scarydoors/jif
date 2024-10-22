struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(
  @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
  var out: VertexOutput;
  var pos: vec2<f32>;

  switch in_vertex_index {
    case 0u: {
      pos = vec2(-1.0, 1.0);
      out.tex_coord = vec2(0.0, 0.0);
    }
    case 1u: {
      pos = vec2(1.0, 1.0);
      out.tex_coord = vec2(1.0, 0.0);
    }
    case 2u: {
      pos = vec2(1.0, -1.0);
      out.tex_coord = vec2(1.0, 1.0);
    }
    case 3u: {
      pos = vec2(-1.0, 1.0);
      out.tex_coord = vec2(0.0, 0.0);
    }
    case 4u: {
      pos = vec2(1.0, -1.0);
      out.tex_coord = vec2(1.0, 1.0);
    }
    case 5u: {
      pos = vec2(-1.0, -1.0);
      out.tex_coord = vec2(0.0, 1.0);
    }
    default: {
      pos = vec2(-1.0, -1.0);
      out.tex_coord = vec2(0.0, 1.0);
    }
  }
  out.clip_position = vec4<f32>(pos, 0.0, 1.0);
  return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(t_diffuse, s_diffuse, in.tex_coord);
}
