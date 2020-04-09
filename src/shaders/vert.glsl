#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;
layout(location = 0) out vec2 tex_coords;

layout(push_constant) uniform Mats {
	mat4 mpv;
} mats;

void main() {
	gl_Position = mats.mpv * vec4(pos, 1.0);
	tex_coords = uv;
}
