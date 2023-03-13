use wgpu::util::DeviceExt;
mod controller;
pub use controller::CameraController;
mod uniform;
pub use uniform::CameraUniform;


pub fn create_camera_buffer(device: &wgpu::Device, camera_uniform: CameraUniform) -> wgpu::Buffer {
	device.create_buffer_init(
		&wgpu::util::BufferInitDescriptor {
			label: Some("Camera Buffer"),
			contents: bytemuck::cast_slice(&[camera_uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		}
	)
}

pub fn create_camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(
		&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility:
						wgpu::ShaderStages::VERTEX |
						wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}
			],
			label: Some("camera_bind_group_layout"),
		}
	)
}

pub fn create_camera_bind_group(device: &wgpu::Device, camera_bind_group_layout: &wgpu::BindGroupLayout, camera_buffer: &wgpu::Buffer) -> wgpu::BindGroup {
    device.create_bind_group(
		&wgpu::BindGroupDescriptor {
			layout: camera_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: camera_buffer.as_entire_binding(),
				}
			],
			label: Some("camera_bind_group"),
		}
	)
}


#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0, 1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

pub fn create_camera(config: &wgpu::SurfaceConfiguration) -> Camera {
	Camera::new(
		(0.0, 5.0, -10.0).into(),
		(0.0, 0.0, 0.0).into(),
		cgmath::Vector3::unit_y(),
		config.width as f32 / config.height as f32,
		45.0,
		0.1,
		100.0,
	)
}

pub struct Camera {
	eye: cgmath::Point3<f32>,
	target: cgmath::Point3<f32>,
	up: cgmath::Vector3<f32>,
	aspect: f32,
	fovy: f32,
	znear: f32,
	zfar: f32,
}

impl Camera {
	pub fn new(
		eye: cgmath::Point3<f32>,
		target: cgmath::Point3<f32>,
		up: cgmath::Vector3<f32>,
		aspect: f32,
		fovy: f32,
		znear: f32,
		zfar: f32,
	) -> Self {

		Self {
			eye,
			target,
			up,
			aspect,
			fovy,
			znear,
			zfar
		}
	}

	fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
		let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
		let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
		
		proj * view
	}
}