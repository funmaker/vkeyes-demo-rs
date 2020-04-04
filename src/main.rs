mod openvr_vulkan;

use std::error::Error;

use vulkano::app_info_from_cargo_toml;
use vulkano::instance::{InstanceExtensions, RawInstanceExtensions, Instance, PhysicalDevice};
use vulkano::instance::debug::{MessageSeverity, MessageType, DebugCallback};
use vulkano::device::{Device, RawDeviceExtensions, DeviceExtensions, Features};
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::format;
use vulkano::format::ClearValue;
use vulkano::image::{AttachmentImage, ImageUsage, ImageAccess};

use openvr_vulkan::{OpenVRPtr, vulkan_device_extensions_required};
use openvr::compositor::Texture;
use openvr::compositor::texture::{ColorSpace, Handle, vulkan};
use openvr::Eye;
use vulkano::buffer::BufferUsage;

#[derive(Default, Copy, Clone)]
struct Vertex {
	position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);



fn main() -> Result<(), Box<dyn Error>> {
	let debug = true;
	
	let context = unsafe { openvr::init(openvr::ApplicationType::Scene) }?;
	let system = context.system()?;
	let _chaperone = context.chaperone()?;
	let compositor = context.compositor()?;
	
	let recommended_size = system.recommended_render_target_size();
	
	let instance = {
		let app_infos = app_info_from_cargo_toml!();
		let extensions = RawInstanceExtensions::new(compositor.vulkan_instance_extensions_required())
		                                       .union(&(&InstanceExtensions { ext_debug_utils: debug,
		                                                                      ..InstanceExtensions::none() }).into());
		
		let layers = if debug {
			             vec!["VK_LAYER_LUNARG_standard_validation"]
		             } else {
			             vec![]
		             };
		
		Instance::new(Some(&app_infos), extensions, layers)?
	};
	
	if debug {
		let severity = MessageSeverity { error:       true,
		                                 warning:     true,
		                                 information: false,
		                                 verbose:     true, };
		
		let ty = MessageType::all();
		
		let _debug_callback = DebugCallback::new(&instance, severity, ty, |msg| {
			                                         let severity = if msg.severity.error {
				                                         "error"
			                                         } else if msg.severity.warning {
				                                         "warning"
			                                         } else if msg.severity.information {
				                                         "information"
			                                         } else if msg.severity.verbose {
				                                         "verbose"
			                                         } else {
				                                         panic!("no-impl");
			                                         };
			                                         
			                                         let ty = if msg.ty.general {
				                                         "general"
			                                         } else if msg.ty.validation {
				                                         "validation"
			                                         } else if msg.ty.performance {
				                                         "performance"
			                                         } else {
				                                         panic!("no-impl");
			                                         };
			                                         
			                                         println!("{} {} {}: {}",
			                                                  msg.layer_prefix,
			                                                  ty,
			                                                  severity,
			                                                  msg.description);
		                                         });
	}
	
	let physical = system.vulkan_output_device(instance.as_ptr())
	                     .and_then(|ptr| PhysicalDevice::enumerate(&instance).find(|physical| physical.as_ptr() == ptr))
	                     .or_else(|| {
		                     println!("Failed to fetch device from openvr, fallback to the first one");
		                     PhysicalDevice::enumerate(&instance).next()
	                     })
	                     .expect("No physical devices found.");
	
	println!("\nUsing {}: {} api: {} driver: {}",
	         physical.index(),
	         physical.name(),
	         physical.api_version(),
	         physical.driver_version());
	
	let queue_family = physical.queue_families()
	                           .find(|queue| queue.supports_graphics() && queue.supports_compute())
	                           .expect("No graphical and compute queue family found for this device.");
	
	let (device, mut queues) = Device::new(physical,
	                                       &Features::none(),
	                                       RawDeviceExtensions::new(vulkan_device_extensions_required(&compositor, &physical))
	                                                           .union(&(&DeviceExtensions { khr_swapchain: true,
	                                                                                        ..DeviceExtensions::none() }).into()),
	                                       [(queue_family, 0.5)].iter().cloned())?;
	
	let queue = queues.next().expect("No queue found for this queue family");
	
	let left_eye = AttachmentImage::with_usage(device.clone(),
	                                           [recommended_size.0, recommended_size.1],
	                                           format::R8G8B8A8Srgb,
	                                           ImageUsage { transfer_source: true,
	                                               transfer_destination: true,
	                                                        sampled: true,
	                                                        ..ImageUsage::none() })?;
	
	let left_eye_texture = Texture {
		handle: Handle::Vulkan(vulkan::Texture {
			image: (*left_eye).as_ptr(),
			device: device.as_ptr(),
			physical_device: physical.as_ptr(),
			instance: instance.as_ptr(),
			queue: queue.as_ptr(),
			queue_family_index: queue_family.id(),
			width: left_eye.dimensions().width(),
			height: left_eye.dimensions().height(),
			format: left_eye.format() as u32,
			sample_count: left_eye.samples(),
		}),
		color_space: ColorSpace::Gamma,
	};
	
	let right_eye = AttachmentImage::with_usage(device.clone(),
	                                           [recommended_size.0, recommended_size.1],
	                                           format::R8G8B8A8Srgb,
	                                           ImageUsage { transfer_source: true,
	                                                        transfer_destination: true,
	                                                        sampled: true,
	                                                        ..ImageUsage::none() })?;
	
	let right_eye_texture = Texture {
		handle: Handle::Vulkan(vulkan::Texture {
			image: right_eye.as_ptr(),
			device: device.as_ptr(),
			physical_device: physical.as_ptr(),
			instance: instance.as_ptr(),
			queue: queue.as_ptr(),
			queue_family_index: queue_family.id(),
			width: right_eye.dimensions().width(),
			height: right_eye.dimensions().height(),
			format: right_eye.format() as u32,
			sample_count: right_eye.samples(),
		}),
		color_space: ColorSpace::Gamma,
	};
	
	let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(),
	                                                   vec![vertex1, vertex2, vertex3].into_iter()).unwrap();
	
	
	let mut previous_frame_end: Option<Box<dyn GpuFuture>> = Some(Box::new(sync::now(device.clone())));
	
	loop {
		let poses = compositor.wait_get_poses()?;
		previous_frame_end.as_mut().unwrap().cleanup_finished();
		
		let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())?
			.clear_color_image(left_eye.clone(), ClearValue::Float([0.176, 1.176, 0.176, 1.0]))?
			.clear_color_image(right_eye.clone(), ClearValue::Float([0.176, 0.176, 0.176, 1.0]))?
			.build()?;
		
		let future = previous_frame_end.take()
		                               .unwrap()
		                               .then_execute(queue.clone(), command_buffer)?;
		
		unsafe {
			compositor.submit(Eye::Left, &left_eye_texture, None, None);
			compositor.submit(Eye::Right, &right_eye_texture, None, None);
		}
		
		let future = future.then_signal_fence_and_flush();
		
		match future {
			Ok(future) => {
				previous_frame_end = Some(Box::new(future) as Box<_>);
			},
			Err(FlushError::OutOfDate) => {
				eprintln!("Flush Error: Out of date, ignoring");
				previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
			},
			Err(err) => {
				previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<_>);
				return Err(err.into());
			},
		}
	}
	
	Ok(())
}
