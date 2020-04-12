use std::collections::HashMap;
use err_derive::Error;
use openvr::{System, Compositor, RenderModels, Context, InitError, tracked_device_index, TrackedDeviceClass, render_models, TrackedControllerRole};
use openvr::compositor::CompositorError;
use openvr::system::TrackedPropertyError;
use image::{ImageError, DynamicImage, ImageBuffer};
use obj::{load_obj, ObjError, TexturedVertex, Obj};
use cgmath::{Matrix4, Vector2};

use crate::renderer::{Renderer, RendererCreationError, RenderError, model};
use crate::renderer::model::{Model, ModelError, Vertex};
use crate::openvr_vulkan::mat4;

pub struct Application {
	context: Context,
	system: System,
	compositor: Compositor,
	render_models: RenderModels,
	renderer: Renderer,
}

impl Application {
	pub fn new(device: Option<usize>, debug: bool) -> Result<Application, ApplicationCreationError> {
		let context = unsafe { openvr::init(openvr::ApplicationType::Scene) }?;
		let system = context.system()?;
		let compositor = context.compositor()?;
		let render_models = context.render_models()?;
		
		let renderer = Renderer::new(&system, context.compositor()?, device, debug)?;
		
		Ok(Application {
			context,
			system,
			compositor,
			render_models,
			renderer,
		})
	}
	
	pub fn run(mut self) -> Result<(), ApplicationRunError> {
		let mut scene = Vec::new();
		
		{
			let obj: Obj<TexturedVertex, u16> = load_obj(model::SCENE_OBJ)?;
			
			let vertices: Vec<Vertex> = obj.vertices.iter().map(Into::into).collect();
			let image = image::load_from_memory(model::SCENE_PNG)?;
			
			let model = Model::new(&vertices, &obj.indices, image, &self.renderer)?;
			
			scene.push((model, Matrix4::from_scale(0.035)));
		}
		
		let mut devices: HashMap<u32, usize> = HashMap::new();
		let mut eye_rotation = (Vector2::new(0.0, 0.0), Vector2::new(0.0, 0.0));
		
		loop {
			let poses = self.compositor.wait_get_poses()?;
			
			for i in 0..poses.render.len() as u32 {
				if self.system.tracked_device_class(i) != TrackedDeviceClass::Invalid
				&& self.system.tracked_device_class(i) != TrackedDeviceClass::HMD {
					if devices.contains_key(&i) {
						scene[*devices.get(&i).unwrap()].1 = mat4(poses.render[i as usize].device_to_absolute_tracking());
					} else if let Some(model) = self.render_models.load_render_model(&self.system.string_tracked_device_property(i, 1003)?)? {
						if let Some(texture) = self.render_models.load_texture(model.diffuse_texture_id().unwrap())? {
							let vertices: Vec<Vertex> = model.vertices().iter().map(Into::into).collect();
							let indices = model.indices();
							let size = texture.dimensions();
							let image = DynamicImage::ImageRgba8(ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture.data().into()).unwrap());
							
							let model = Model::new(&vertices, indices, image, &self.renderer)?;
							
							devices.insert(i, scene.len());
							scene.push((model, mat4(poses.render[i as usize].device_to_absolute_tracking())));
							println!("Loaded {:?}", self.system.tracked_device_class(i));
						} else { break }
					} else { break }
				}
			}
			
			if let Some(i) = self.system.tracked_device_index_for_controller_role(TrackedControllerRole::LeftHand) {
				if let Some(state) = self.system.controller_state(i) {
					eye_rotation.0 += Vector2::new(-state.axis[0].y, state.axis[0].x) / 100.0;
				}
			}

			if let Some(i) = self.system.tracked_device_index_for_controller_role(TrackedControllerRole::RightHand) {
				if let Some(state) = self.system.controller_state(i) {
					eye_rotation.1 += Vector2::new(-state.axis[0].y, state.axis[0].x) / 100.0;
				}
			}
			
			let pose = poses.render[tracked_device_index::HMD as usize].device_to_absolute_tracking();
			
			self.renderer.render(pose, eye_rotation, &mut scene)?;
		}
		
		// Ok(())
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
	#[error(display = "{}", _0)] CompositorError(#[error(source)] CompositorError),
	#[error(display = "{}", _0)] RenderError(#[error(source)] RenderError),
	#[error(display = "{}", _0)] TrackedPropertyError(#[error(source)] TrackedPropertyError),
	#[error(display = "{}", _0)] RenderModelError(#[error(source)] render_models::Error),
	#[error(display = "{}", _0)] ObjError(#[error(source)] ObjError),
}
