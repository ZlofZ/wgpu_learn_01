
use super::Camera;

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	// We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
	view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
	pub fn new() -> Self {
		use cgmath::SquareMatrix;
		Self {
			view_position: [0.0; 4],
			view_proj: cgmath::Matrix4::identity().into(),
		}
	}

	pub fn update_view_proj(&mut self, camera: &Camera) {
		self.view_position = camera.eye.to_homogeneous().into();
		self.view_proj = camera.build_view_projection_matrix().into();
	}

}

