
#[derive(Default, Copy, Clone)]
pub struct Vertex {
	pos: [f32; 3],
	uv: [f32; 2],
}

vulkano::impl_vertex!(Vertex, pos, uv);

impl Vertex {
	pub const fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
		Vertex {
			pos: [x, y, z],
			uv: [u, v],
		}
	}
}

pub(crate) const CUBE: [Vertex; 36] = [
	// left
	Vertex::new(-1.0, -1.0, -1.0, 1.0, 0.0),
	Vertex::new(-1.0,  1.0,  1.0, 0.0, 1.0),
	Vertex::new(-1.0, -1.0,  1.0, 0.0, 0.0),
	Vertex::new(-1.0,  1.0,  1.0, 0.0, 1.0),
	Vertex::new(-1.0, -1.0, -1.0, 1.0, 0.0),
	Vertex::new(-1.0,  1.0, -1.0, 1.0, 1.0),
	// back
	Vertex::new(-1.0, -1.0, -1.0, 0.0, 0.0),
	Vertex::new( 1.0, -1.0, -1.0, 1.0, 0.0),
	Vertex::new( 1.0,  1.0, -1.0, 1.0, 1.0),
	Vertex::new(-1.0, -1.0, -1.0, 0.0, 0.0),
	Vertex::new( 1.0,  1.0, -1.0, 1.0, 1.0),
	Vertex::new(-1.0,  1.0, -1.0, 0.0, 1.0),
	// down
	Vertex::new(-1.0, -1.0, -1.0, 0.0, 1.0),
	Vertex::new( 1.0, -1.0,  1.0, 1.0, 0.0),
	Vertex::new( 1.0, -1.0, -1.0, 1.0, 1.0),
	Vertex::new(-1.0, -1.0, -1.0, 0.0, 1.0),
	Vertex::new(-1.0, -1.0,  1.0, 0.0, 0.0),
	Vertex::new( 1.0, -1.0,  1.0, 1.0, 0.0),
	// up
	Vertex::new(-1.0,  1.0, -1.0, 0.0, 0.0),
	Vertex::new( 1.0,  1.0,  1.0, 1.0, 1.0),
	Vertex::new(-1.0,  1.0,  1.0, 0.0, 1.0),
	Vertex::new(-1.0,  1.0, -1.0, 0.0, 0.0),
	Vertex::new( 1.0,  1.0, -1.0, 1.0, 0.0),
	Vertex::new( 1.0,  1.0,  1.0, 1.0, 1.0),
	// right
	Vertex::new( 1.0,  1.0, -1.0, 0.0, 1.0),
	Vertex::new( 1.0, -1.0,  1.0, 1.0, 0.0),
	Vertex::new( 1.0,  1.0,  1.0, 1.0, 1.0),
	Vertex::new( 1.0, -1.0,  1.0, 1.0, 0.0),
	Vertex::new( 1.0,  1.0, -1.0, 0.0, 1.0),
	Vertex::new( 1.0, -1.0, -1.0, 0.0, 0.0),
	// front
	Vertex::new(-1.0,  1.0,  1.0, 1.0, 1.0),
	Vertex::new( 1.0,  1.0,  1.0, 0.0, 1.0),
	Vertex::new(-1.0, -1.0,  1.0, 1.0, 0.0),
	Vertex::new(-1.0, -1.0,  1.0, 1.0, 0.0),
	Vertex::new( 1.0,  1.0,  1.0, 0.0, 1.0),
	Vertex::new( 1.0, -1.0,  1.0, 0.0, 0.0),
];
