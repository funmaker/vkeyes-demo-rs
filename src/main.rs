mod openvr_vulkan;

use std::error::Error;
use std::sync::Arc;

use vulkano::app_info_from_cargo_toml;
use vulkano::instance::{InstanceExtensions, RawInstanceExtensions, Instance, PhysicalDevice, QueueFamily};
use vulkano::instance::debug::{MessageSeverity, MessageType, DebugCallback};
use vulkano::device::{Device, RawDeviceExtensions, DeviceExtensions, Features, Queue};
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::format;
use vulkano::image::{AttachmentImage, ImageUsage, ImageAccess, ImmutableImage, Dimensions};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::pipeline::viewport::Viewport;
use vulkano::framebuffer::{Subpass, Framebuffer};
use vulkano::format::{ClearValue, Format};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;

use openvr_vulkan::{OpenVRPtr, vulkan_device_extensions_required};
use openvr::compositor::Texture;
use openvr::compositor::texture::{ColorSpace, Handle, vulkan};
use openvr::Eye;

use cgmath::{Matrix4, Matrix, Transform};

use image::GenericImageView;
use vulkano::sampler::Sampler;

#[derive(Default, Copy, Clone)]
struct Vertex {
	pos: [f32; 3],
	uv: [f32; 2],
}

vulkano::impl_vertex!(Vertex, pos, uv);

impl Vertex {
	const fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
		Vertex {
			pos: [x, y, z],
			uv: [u, v],
		}
	}
}

const CUBE: [Vertex; 36] = [
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

const CLIP: Matrix4<f32> = Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0,-1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

mod vs {
	vulkano_shaders::shader!{
		ty: "vertex",
		src: "
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
}"
	}
}

mod fs {
	vulkano_shaders::shader!{
		ty: "fragment",
		src: "
#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
    f_color = texture(tex, tex_coords);
}"
	}
}

fn create_eye_image(device: &Arc<Device>, recommended_size: (u32, u32), physical: &PhysicalDevice, instance: &Instance, queue: &Queue, queue_family: &QueueFamily)
-> Result<( Arc<AttachmentImage<format::R8G8B8A8Srgb>>,
            Arc<AttachmentImage<format::D16Unorm>>,
            Texture ),
          Box<dyn Error>> {
	let dimensions = [recommended_size.0, recommended_size.1];
	
	let image = AttachmentImage::with_usage(device.clone(),
	                                        dimensions,
	                                        format::R8G8B8A8Srgb,
	                                        ImageUsage { transfer_source: true,
	                                                     transfer_destination: true,
	                                                     sampled: true,
	                                                     ..ImageUsage::none() })?;
	
	let depth_buffer = AttachmentImage::transient(device.clone(), dimensions, format::D16Unorm)?;
	
	let texture = Texture {
		handle: Handle::Vulkan(vulkan::Texture {
			image: (*image).as_ptr(),
			device: device.as_ptr(),
			physical_device: physical.as_ptr(),
			instance: instance.as_ptr(),
			queue: queue.as_ptr(),
			queue_family_index: queue_family.id(),
			width: image.dimensions().width(),
			height: image.dimensions().height(),
			format: image.format() as u32,
			sample_count: image.samples(),
		}),
		color_space: ColorSpace::Gamma,
	};
	
	Ok((image, depth_buffer, texture))
}

fn mat4(val: &[[f32; 4]; 3]) -> Matrix4<f32> {
	let mat: Matrix4<f32> = [val[0], val[1], val[2], [0.0, 0.0, 0.0, 1.0]].into();
	mat.transpose()
}

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
	
	let (left_eye,  left_eye_depth,  left_eye_texture ) = create_eye_image(&device, recommended_size, &physical, &instance, &queue, &queue_family)?;
	let (right_eye, right_eye_depth, right_eye_texture) = create_eye_image(&device, recommended_size, &physical, &instance, &queue, &queue_family)?;
	
	let (vertex_buffer, vertex_future) = {
		let cubes: Vec<Vertex> = (0 .. 11*11*11).flat_map(|id| CUBE.iter().map(move |ver| {
			let x = (id % 11 - 5) as f32 * 2.0;
			let y = (id / 11 % 11 - 5) as f32 * 2.0;
			let z = (id / 11 / 11 % 11 - 5) as f32 * 2.0;
			
			Vertex::new(ver.pos[0] * 0.20 + x, ver.pos[1] * 0.20 + y + 1.0, ver.pos[2] * 0.20 + z, ver.uv[0], ver.uv[1])
		})).collect();
		
		ImmutableBuffer::from_iter(cubes.into_iter(),
		                           BufferUsage{ vertex_buffer: true, ..BufferUsage::none() },
		                           queue.clone())?
	};
	
	
	let vs = vs::Shader::load(device.clone()).unwrap();
	let fs = fs::Shader::load(device.clone()).unwrap();
	
	let render_pass = Arc::new(
		vulkano::single_pass_renderpass!(device.clone(),
			attachments: {
				color: {
					load: Clear,
					store: Store,
					format: left_eye.format(),
					samples: 1,
				},
				depth: {
					load: Clear,
					store: DontCare,
					format: left_eye_depth.format(),
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {depth}
			}
		)?
	);
	
	let left_eye_fb = Arc::new(Framebuffer::start(render_pass.clone())
	                                       .add(left_eye)?
	                                       .add(left_eye_depth)?
	                                       .build()?);
	
	let right_eye_fb = Arc::new(Framebuffer::start(render_pass.clone())
	                                        .add(right_eye)?
	                                        .add(right_eye_depth)?
	                                        .build()?);
	
	let pipeline = Arc::new(
		GraphicsPipeline::start()
		                 .vertex_input_single_buffer::<Vertex>()
		                 .vertex_shader(vs.main_entry_point(), ())
		                 .viewports(Some(Viewport { origin: [0.0, 0.0],
		                                            dimensions: [recommended_size.0 as f32, recommended_size.1 as f32],
		                                            depth_range: 0.0 .. 1.0 }))
		                 .fragment_shader(fs.main_entry_point(), ())
		                 .depth_stencil_simple_depth()
		                 .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
		                 .build(device.clone())?
	);
	
	let (image, image_promise) = {
		let image = image::load_from_memory(include_bytes!("../assets/cube_texture.png"))?;
		
		let width = image.width();
		let height = image.height();
		
		ImmutableImage::from_iter(image.to_rgba().into_vec().into_iter(),
		                          Dimensions::Dim2d{ width, height },
		                          Format::R8G8B8A8Unorm,
		                          queue.clone())?
	};
	
	let sampler = Sampler::simple_repeat_linear_no_mipmap(device.clone());
	let layout = pipeline.layout().descriptor_set_layout(0).unwrap();
	
	let set = Arc::new(
		PersistentDescriptorSet::start(layout.clone())
		                        .add_sampled_image(image.clone(), sampler.clone())?
		                        .build()?
	);
	
	let proj_left : Matrix4<f32> = system.projection_matrix(Eye::Left,  0.1, 1000.1).into();
	let proj_right: Matrix4<f32> = system.projection_matrix(Eye::Right, 0.1, 1000.1).into();
	let proj_left  = CLIP * proj_left.transpose();
	let proj_right = CLIP * proj_right.transpose();
	
	let assets_promise = vertex_future.join(image_promise);
	let mut previous_frame_end: Option<Box<dyn GpuFuture>> = Some(Box::new(assets_promise));
	
	loop {
		let poses = compositor.wait_get_poses()?;
		previous_frame_end.as_mut().unwrap().cleanup_finished();
		
		let pose = poses.render[0].device_to_absolute_tracking();
		
		let left_mpv  = proj_left  * (mat4(pose) * mat4(&system.eye_to_head_transform(Eye::Left ))).inverse_transform().unwrap();
		let right_mpv = proj_right * (mat4(pose) * mat4(&system.eye_to_head_transform(Eye::Right))).inverse_transform().unwrap();
		
		let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())?
			.begin_render_pass(left_eye_fb.clone(),
			                   false,
			                   vec![ [0.5, 0.5, 0.5, 1.0].into(),
			                         ClearValue::Depth(1.0) ])?
			.draw(pipeline.clone(),
			      &DynamicState::none(),
			      vertex_buffer.clone(),
			      set.clone(),
			      left_mpv)?
			.end_render_pass()?
			.begin_render_pass(right_eye_fb.clone(),
			                   false,
			                   vec![ [0.5, 0.5, 0.5, 1.0].into(),
			                         ClearValue::Depth(1.0) ])?
			.draw(pipeline.clone(),
			      &DynamicState::none(),
			      vertex_buffer.clone(),
			      set.clone(),
			      right_mpv)?
			.end_render_pass()?
			.build()?;
		
		let future = previous_frame_end.take()
		                               .unwrap()
		                               .then_execute(queue.clone(), command_buffer)?;
		
		unsafe {
			compositor.submit(Eye::Left,  &left_eye_texture,  None, Some(pose.clone()))?;
			compositor.submit(Eye::Right, &right_eye_texture, None, Some(pose.clone()))?;
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
				return Err(err.into());
			},
		}
	}
	
	// Ok(())
}
