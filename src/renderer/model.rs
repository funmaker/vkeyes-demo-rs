use std::sync::Arc;
use std::time::Duration;
use err_derive::Error;
use image::{DynamicImage, GenericImageView};
use vulkano::buffer::{ImmutableBuffer, BufferUsage};
use vulkano::image::{ImmutableImage, Dimensions, ImageCreationError};
use vulkano::sync::{GpuFuture, FlushError, FenceSignalFuture};
use vulkano::format::Format;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::sampler::Sampler;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet, PersistentDescriptorSetError, PersistentDescriptorSetBuildError};
use vulkano::descriptor::PipelineLayoutAbstract;
use arc_swap::ArcSwap;

use crate::renderer::Renderer;

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

pub const SCENE_OBJ: &[u8] = include_bytes!("../../assets/scene.obj");
pub const SCENE_PNG: &[u8] = include_bytes!("../../assets/scene.png");

#[derive(Clone)]
pub struct Model {
	pub buffer: Arc<ImmutableBuffer<[Vertex]>>,
	pub image: Arc<ImmutableImage<Format>>,
	pub set: Arc<dyn DescriptorSet + Send + Sync>,
	fence: ArcSwap<FenceCheck>,
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
			PersistentDescriptorSet::start(renderer.pipeline.descriptor_set_layout(0).ok_or(ModelError::NoLayout)?.clone())
			                        .add_sampled_image(image.clone(), sampler.clone())?
			                        .build()?
		);
		
		let fence = ArcSwap::new(Arc::new(FenceCheck::new(buffer_promise.join(image_promise))?));
		
		Ok(Model {
			buffer,
			image,
			set,
			fence,
		})
	}
	
	pub fn loaded(&self) -> bool {
		match &**self.fence.load() {
			FenceCheck::Done(result) => *result,
			FenceCheck::Pending(fence) => {
				match fence.wait(Some(Duration::new(0, 0))) {
					Err(FlushError::Timeout) => false,
					Ok(()) => {
						self.fence.swap(Arc::new(FenceCheck::Done(true)));
						true
					}
					Err(err) => {
						eprintln!("Error while loading model: {:?}", err);
						self.fence.swap(Arc::new(FenceCheck::Done(false)));
						false
					}
				}
			}
		}
	}
}

enum FenceCheck {
	Done(bool),
	Pending(FenceSignalFuture<Box<dyn GpuFuture>>)
}

impl FenceCheck {
	fn new<GF>(future: GF)
	          -> Result<FenceCheck, FlushError>
	          where GF: GpuFuture + 'static {
		Ok(FenceCheck::Pending((Box::new(future) as Box<dyn GpuFuture>).then_signal_fence_and_flush()?))
	}
}


#[derive(Debug, Error)]
pub enum ModelError {
	#[error(display = "Pipeline doesn't have layout set 0")] NoLayout,
	#[error(display = "{}", _0)] DeviceMemoryAllocError(#[error(source)] DeviceMemoryAllocError),
	#[error(display = "{}", _0)] ImageCreationError(#[error(source)] ImageCreationError),
	#[error(display = "{}", _0)] FlushError(#[error(source)] FlushError),
	#[error(display = "{}", _0)] PersistentDescriptorSetError(#[error(source)] PersistentDescriptorSetError),
	#[error(display = "{}", _0)] PersistentDescriptorSetBuildError(#[error(source)] PersistentDescriptorSetBuildError),
}
