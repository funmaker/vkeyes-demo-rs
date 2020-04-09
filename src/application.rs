use openvr::{System, Compositor, RenderModels, Chaperone, Context, InitError, Eye};
use err_derive::Error;
use image::ImageError;
use cgmath::{Matrix4, Matrix};

use crate::renderer::{Renderer, RendererCreationError};
use crate::renderer::model::{Model, ModelError};
use crate::models;

pub struct Application {
	context: Context,
	system: System,
	compositor: Compositor,
	render_models: RenderModels,
	chaperone: Chaperone,
	renderer: Renderer,
}

const CLIP: Matrix4<f32> = Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0,-1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

fn mat4(val: &[[f32; 4]; 3]) -> Matrix4<f32> {
	let mat: Matrix4<f32> = [val[0], val[1], val[2], [0.0, 0.0, 0.0, 1.0]].into();
	mat.transpose()
}

impl Application {
	pub fn new(device: Option<usize>, debug: bool) -> Result<Application, ApplicationCreationError> {
		let context = unsafe { openvr::init(openvr::ApplicationType::Scene) }?;
		let system = context.system()?;
		let compositor = context.compositor()?;
		let render_models = context.render_models()?;
		let chaperone = context.chaperone()?;
		
		let renderer = Renderer::new(&system, &compositor, device, debug)?;
		
		Ok(Application {
			context,
			system,
			compositor,
			render_models,
			chaperone,
			renderer,
		})
	}
	
	pub fn run(mut self) -> Result<(), ApplicationRunError> {
		let image = image::load_from_memory(include_bytes!("../assets/cube_texture.png"))?;
		let cube = Model::new(&models::CUBE, image, &self.renderer)?;
		
		let hmd_index = openvr::tracked_device_index::HMD;
		
		let mut scene: Vec<(Model, Matrix4<f32>)> = (0 .. 11 * 11 * 11).map(|i| {
			let x = (i % 11) * 3.0;
			let y = (i / 11 % 11) * 3.0;
			let z = (i / 11 / 11 % 11) * 3.0;
			
			(cube.clone(), Matrix4::new(0.2, 0.0, 0.0, 0.0,
			                            0.0, 0.2, 0.0, 0.0,
			                            0.0, 0.0, 0.2, 0.0,
			                              x,   y,   z, 1.0));
		}).collect();
		
		let proj_left : Matrix4<f32> = self.system.projection_matrix(Eye::Left,  0.1, 1000.1).into();
		let proj_right: Matrix4<f32> = self.system.projection_matrix(Eye::Right, 0.1, 1000.1).into();
		let proj_left  = CLIP * proj_left.transpose();
		let proj_right = CLIP * proj_right.transpose();
		
		loop {
			let poses = self.compositor.wait_get_poses()?;
			
			let pose = poses.render[hmd_index].device_to_absolute_tracking();
			
			let left_pv  = proj_left  * (mat4(pose) * mat4(&self.system.eye_to_head_transform(Eye::Left ))).inverse_transform().unwrap();
			let right_pv = proj_right * (mat4(pose) * mat4(&self.system.eye_to_head_transform(Eye::Right))).inverse_transform().unwrap();
			
			self.renderer.render(&self.compositor, pose, left_pv, right_pv, &mut scene)?;
		}
		
		Ok(())
	}
}

impl Drop for Application {
	fn drop(&mut self) {
		// Context has to be shutdown before dropping graphical API
		unsafe { self.context.shutdown(); }
	}
}

#[derive(Debug, Error)]
pub enum ApplicationCreationError {
	#[error(display = "{}", _0)] OpenVRInitError(#[error(source)] InitError),
	#[error(display = "{}", _0)] RendererCreationError(#[error(source)] RendererCreationError),
}

#[derive(Debug, Error)]
pub enum ApplicationRunError {
	#[error(display = "{}", _0)] ImageError(#[error(source)] ImageError),
	#[error(display = "{}", _0)] ModelError(#[error(source)] ModelError),
}
