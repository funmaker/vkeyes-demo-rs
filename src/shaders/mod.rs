
pub mod vert {
	vulkano_shaders::shader! {
		ty: "vertex",
		path: "src/shaders/vert.glsl"
	}
}

pub mod frag {
	vulkano_shaders::shader! {
		ty: "fragment",
		path: "src/shaders/frag.glsl"
	}
}
