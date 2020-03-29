use std::mem;

use vulkano::instance::Instance;
use vulkano::VulkanObject;
use openvr::VkInstance_T;

pub trait OpenVRPtr {
	type PtrType;
	
	fn as_ptr(&self) -> Self::PtrType;
	fn from_ptr(ptr: Self::PtrType) -> Self;
}

impl OpenVRPtr for Instance {
	type PtrType = *mut VkInstance_T;
	
	fn as_ptr(&self) -> Self::PtrType {
		self.internal_object() as Self::PtrType
	}
	
	fn from_ptr(ptr: Self::PtrType) -> Self {
	
	}
}
