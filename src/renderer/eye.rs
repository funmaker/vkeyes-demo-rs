use std::sync::Arc;
use err_derive::Error;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, FramebufferCreationError, RenderPassAbstract};
use vulkano::image::{AttachmentImage, ImageUsage, ImageAccess, ImageCreationError};
use vulkano::format;
use vulkano::format::Format;
use vulkano::device::Queue;
use openvr::compositor::Texture;
use openvr::compositor::texture::{vulkan, Handle, ColorSpace};
use cgmath::Matrix4;

use crate::openvr_vulkan::OpenVRPtr;

pub struct Eye {
	pub image: Arc<AttachmentImage<format::R8G8B8A8Srgb>>,
	pub depth_image: Arc<AttachmentImage<format::D16Unorm>>,
	pub texture: Texture,
	pub projection: Matrix4<f32>,
	pub frame_buffer: Arc<dyn FramebufferAbstract + Send + Sync>,
}

pub const IMAGE_FORMAT: Format = Format::R8G8B8A8Srgb;
pub const DEPTH_FORMAT: Format = Format::D16Unorm;

impl Eye {
	pub fn new<RPD>(recommended_size:(u32, u32), projection: Matrix4<f32>, queue: &Queue, render_pass: &Arc<RPD>)
	               -> Result<Eye, EyeCreationError>
	               where RPD: RenderPassAbstract + Sync + Send + 'static {
		let dimensions = [recommended_size.0, recommended_size.1];
		
		let device = queue.device();
		
		let image = AttachmentImage::with_usage(device.clone(),
		                                        dimensions,
		                                        format::R8G8B8A8Srgb,
		                                        ImageUsage { transfer_source: true,
		                                                     transfer_destination: true,
		                                                     sampled: true,
		                                                     ..ImageUsage::none() })?;
		
		let depth_image = AttachmentImage::transient(device.clone(), dimensions, format::D16Unorm)?;
		
		let texture = Texture {
			handle: Handle::Vulkan(vulkan::Texture {
				        image: (*image).as_ptr(),
				        device: device.as_ptr(),
				        physical_device: device.physical_device().as_ptr(),
				        instance: device.instance().as_ptr(),
				        queue: queue.as_ptr(),
				        queue_family_index: queue.family().id(),
				        width: image.dimensions().width(),
				        height: image.dimensions().height(),
				        format: image.format() as u32,
				        sample_count: image.samples(),
			        }),
			color_space: ColorSpace::Gamma,
		};
		
		
		let frame_buffer = Arc::new(Framebuffer::start(render_pass.clone())
		                       .add(image.clone())?
		                       .add(depth_image.clone())?
		                       .build()?);
		
		Ok(Eye {
			image,
			depth_image,
			texture,
			projection,
			frame_buffer,
		})
	}
}

#[derive(Debug, Error)]
pub enum EyeCreationError {
	#[error(display = "{}", _0)] ImageCreationError(#[error(source)] ImageCreationError),
	#[error(display = "{}", _0)] FramebufferCreationError(#[error(source)] FramebufferCreationError),
}
