use openvr::{System, Compositor, RenderModels, Chaperone, Context, InitError, tracked_device_index, TrackedDeviceClass, render_models};
use err_derive::Error;
use image::{ImageError, DynamicImage, ImageBuffer};
use cgmath::Matrix4;

use crate::renderer::{Renderer, RendererCreationError, RenderError};
use crate::renderer::model::{Model, ModelError};
use crate::models;
use crate::models::Vertex;
use crate::openvr_vulkan::mat4;
use openvr::compositor::CompositorError;
use std::collections::HashMap;
use openvr::system::TrackedPropertyError;
use obj::{load_obj, ObjError, TexturedVertex, Obj};

pub struct Application {
	context: Context,
	system: System,
	compositor: Compositor,
	render_models: RenderModels,
	chaperone: Chaperone,
	renderer: Renderer,
}

const SIZE: i32 = 5;

impl Application {
	pub fn new(device: Option<usize>, debug: bool) -> Result<Application, ApplicationCreationError> {
		let context = unsafe { openvr::init(openvr::ApplicationType::Scene) }?;
		let system = context.system()?;
		let compositor = context.compositor()?;
		let render_models = context.render_models()?;
		let chaperone = context.chaperone()?;
		
		let renderer = Renderer::new(&system, context.compositor()?, device, debug)?;
		
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
		
		let mut scene: Vec<(Model, Matrix4<f32>)> = (0 .. SIZE * SIZE * SIZE).map(|i| {
			let x = (i % SIZE - SIZE/2) as f32 * 3.0;
			let y = (i / SIZE % SIZE - SIZE/2) as f32 * 3.0;
			let z = (i / SIZE / SIZE % SIZE - SIZE/2) as f32 * 3.0;
			
			(cube.clone(), Matrix4::new(0.2, 0.0, 0.0, 0.0,
			                            0.0, 0.2, 0.0, 0.0,
			                            0.0, 0.0, 0.2, 0.0,
			                              x,   y,   z, 1.0))
		}).collect();
		
		{
			let obj: Obj<TexturedVertex, usize> = load_obj(&include_bytes!("../assets/scene.obj")[..])?;
			let verticles = obj.indices.iter()
			                           .map(|&i| Vertex::new(
				                           obj.vertices[i].position[0],
				                           obj.vertices[i].position[1],
				                           obj.vertices[i].position[2],
				                           obj.vertices[i].texture[0],
				                           1.0 - obj.vertices[i].texture[1],
			                           ))
			                           .collect::<Vec<Vertex>>();
			let image = image::load_from_memory(include_bytes!("../assets/scene.png"))?;
			let model = Model::new(&verticles, image, &self.renderer)?;
			scene.push((model, Matrix4::new(0.035, 0.0, 0.0, 0.0,
			                                0.0, 0.035, 0.0, 0.0,
			                                0.0, 0.0, 0.035, 0.0,
			                                0.0, 0.0, 0.0, 1.0)));
		}
		
		let mut devices: HashMap<u32, usize> = HashMap::new();
		
		loop {
			let poses = self.compositor.wait_get_poses()?;
			
			for i in 0..poses.render.len() as u32 {
				if self.system.tracked_device_class(i) != TrackedDeviceClass::Invalid
				&& self.system.tracked_device_class(i) != TrackedDeviceClass::HMD {
					if devices.contains_key(&i) {
						scene[*devices.get(&i).unwrap()].1 = mat4(poses.render[i as usize].device_to_absolute_tracking());
					} else if let Some(model) = self.render_models.load_render_model(&self.system.string_tracked_device_property(i, 1003)?)? {
						if let Some(texture) = self.render_models.load_texture(model.diffuse_texture_id().unwrap())? {
							let raw_verts = model.vertices();
							let verticles = model.indices()
							                     .iter()
							                     .map(|&i| Vertex::new(
								                     raw_verts[i as usize].position[0],
								                     raw_verts[i as usize].position[1],
								                     raw_verts[i as usize].position[2],
								                     raw_verts[i as usize].texture_coord[0],
								                     raw_verts[i as usize].texture_coord[1],
							                     ))
							                     .collect::<Vec<Vertex>>();
							
							let size = texture.dimensions();
							let image = DynamicImage::ImageRgba8(ImageBuffer::from_raw(size.0 as u32, size.1 as u32, texture.data().into()).unwrap());
							
							let model = Model::new(&verticles, image, &self.renderer)?;
							
							devices.insert(i, scene.len());
							scene.push((model, mat4(poses.render[i as usize].device_to_absolute_tracking())));
							println!("Loaded {:?}", self.system.tracked_device_class(i));
						} else { break }
					} else { break }
				}
			}
			
			let pose = poses.render[tracked_device_index::HMD as usize].device_to_absolute_tracking();
			
			self.renderer.render(pose, &mut scene)?;
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
