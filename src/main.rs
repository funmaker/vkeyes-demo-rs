mod openvr_vulkan;

use std::error::Error;

use vulkano::{app_info_from_cargo_toml, VulkanObject};
use vulkano::instance::{InstanceExtensions, RawInstanceExtensions, Instance};
use vulkano::instance::debug::{MessageSeverity, MessageType, DebugCallback};

use openvr_vulkan::OpenVRPtr;

fn main() -> Result<(), Box<dyn Error>> {
	let debug = true;
	
	let context = unsafe { openvr::init(openvr::ApplicationType::Scene) }?;
	let system = context.system()?;
	let chaperone = context.chaperone()?;
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
	                     .map()
	                     .or_else(|| {
		                     println!("Failed to fetch device from openvr, fallback to the first one");
		                     
	                     });
	
	Ok(())
}
