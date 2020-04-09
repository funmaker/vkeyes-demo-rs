use std::sync::Arc;
use std::time::Duration;

use err_derive::Error;
use image::{DynamicImage, GenericImageView};
use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::image::{ImmutableImage, Dimensions, ImageCreationError};
use vulkano::sync::{GpuFuture, FenceSignalFuture, FlushError};
use vulkano::format::Format;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::sampler::Sampler;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};

use crate::models::Vertex;
use crate::renderer::Renderer;
use vulkano::descriptor::PipelineLayoutAbstract;

#[derive(Clone)]
pub struct Model {
	pub buffer: Arc<ImmutableBuffer<[Vertex]>>,
	pub image: Arc<ImmutableImage<Format>>,
	pub set: Arc<dyn DescriptorSet>,
	fence: Box<dyn AbstractFenceCheck>,
}

impl Model {
	pub fn new(vertexes: &[Vertex], source_image: DynamicImage, renderer: &Renderer) -> Result<Model, ModelError> {
		let width = source_image.width();
		let height = source_image.height();
		let queue = &renderer.load_queue;
		
		let (buffer, buffer_promise) = ImmutableBuffer::from_iter(vertexes.iter().cloned(),
		                                                          BufferUsage{ vertex_buffer: true, ..BufferUsage::none() },
		                                                          queue.clone())?;
		
		let (image, image_promise) = ImmutableImage::from_iter(source_image.to_rgba().into_vec().into_iter(),
		                                                       Dimensions::Dim2d{ width, height },
		                                                       Format::R8G8B8A8Unorm,
		                                                       queue.clone())?;
		
		let sampler = Sampler::simple_repeat_linear_no_mipmap(queue.device().clone());
		
		let set = Arc::new(
			PersistentDescriptorSet::start(renderer.pipeline.descriptor_set_layout()?.clone())
			                        .add_sampled_image(image.clone(), sampler.clone())?
			                        .build()?
		);
		
		let fence = Box::new(FenceCheck::Pending(buffer_promise.join(image_promise).then_signal_fence_and_flush()?)) as Box<dyn AbstractFenceCheck>;
		
		Ok(Model {
			buffer,
			image,
			set,
			fence,
		})
	}
	
	pub fn loaded(&mut self) -> bool {
		self.fence.loaded()
	}
}

enum FenceCheck<GF: GpuFuture> {
	Done(bool),
	Pending(FenceSignalFuture<GF>)
}

trait AbstractFenceCheck {
	fn loaded(&mut self) -> bool;
}

impl<GF: GpuFuture> AbstractFenceCheck for FenceCheck<GF> {
	fn loaded(&mut self) -> bool {
		match self {
			FenceCheck::Done(result) => *result,
			FenceCheck::Pending(fence) => {
				match fence.wait(Some(Duration::new(0, 0))) {
					Err(FlushError::Timeout) => false,
					Ok(()) => {
						std::mem::replace(self, FenceCheck::Done(true));
						true
					}
					Err(err) => {
						eprintln!("Error while loading model: {:?}", err);
						std::mem::replace(self, FenceCheck::Done(false));
						false
					}
				}
			}
		}
	}
}

#[derive(Debug, Error)]
pub enum ModelError {
	#[error(display = "{}", _0)] DeviceMemoryAllocError(#[error(source)] DeviceMemoryAllocError),
	#[error(display = "{}", _0)] ImageCreationError(#[error(source)] ImageCreationError),
	#[error(display = "{}", _0)] FlushError(#[error(source)] FlushError),
}
