use camera_controller::CameraController;
//use wgpu::RenderPipeline;
use wgpu::util::DeviceExt;
use winit::window::Window;
use cgmath::prelude::*;



mod resources;
mod texture;
mod model;
mod camera_controller;

use model::{Vertex, DrawModel};

// lib.rs

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
	1.0, 0.0, 0.0, 0.0,
	0.0, 1.0, 0.0, 0.0,
	0.0, 0.0, 0.5, 0.0,
	0.0, 0.0, 0.5, 1.0,
);

struct Camera {
	eye: cgmath::Point3<f32>,
	target: cgmath::Point3<f32>,
	up: cgmath::Vector3<f32>,
	aspect: f32,
	fovy: f32,
	znear: f32,
	zfar: f32,
}

impl Camera {

	fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
		let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
		let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
		
		OPENGL_TO_WGPU_MATRIX * proj * view
	}
}
// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
	// We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
	fn new() -> Self {
		Self {
			view_proj: cgmath::Matrix4::identity().into(),
		}
	}

	fn update_view_proj(&mut self, camera: &Camera) {
		self.view_proj = camera.build_view_projection_matrix().into();
	}
}

struct Instance {
	position: cgmath::Vector3<f32>,
	rotation: cgmath::Quaternion<f32>,
}

impl Instance {
	fn to_raw(&self) -> InstanceRaw {
		InstanceRaw {
			model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
		}
	}
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
	model: [[f32; 4]; 4],
}

impl InstanceRaw {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
					shader_location: 5,
					format: wgpu::VertexFormat::Float32x4,
				},
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 6,
					format: wgpu::VertexFormat::Float32x4,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
					shader_location: 7,
					format: wgpu::VertexFormat::Float32x4,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
					shader_location: 8,
					format: wgpu::VertexFormat::Float32x4,
				},
			]
		}
	}
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    bg_color: wgpu::Color,
	render_pipeline: wgpu::RenderPipeline,
	camera: Camera,
	camera_uniform: CameraUniform,
	camera_buffer: wgpu::Buffer,
	camera_bind_group: wgpu::BindGroup,
	camera_controller: camera_controller::CameraController,
	instances: Vec<Instance>,
	instance_buffer: wgpu::Buffer,
	depth_texture: texture::Texture,
	obj_model: model::Model,
}

impl State {
	
    // Creating some of the wgpu types requires async code
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        	backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        
        let adapter = instance.request_adapter(
        	&wgpu::RequestAdapterOptions {
        		power_preference: wgpu::PowerPreference::default(),
        		compatible_surface: Some(&surface),
        		force_fallback_adapter: false,
        	},
        ).await.unwrap();
        
        let (device, queue) = adapter.request_device(
        	&wgpu::DeviceDescriptor {
        		features: wgpu::Features::empty(),
        		// WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                	wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                	wgpu::Limits::default()
                },
                label: None,
        	},
        	None, //Trace path
        ).await.unwrap();
        
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
        	.copied()
        	.filter(|f| f.describe().srgb)
        	.next()
        	.unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
        	usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        	format: surface_format,
        	width: size.width,
        	height: size.height,
        	present_mode: surface_caps.present_modes[0],
        	alpha_mode: surface_caps.alpha_modes[0],
        	view_formats: vec![],
        };
        surface.configure(&device, &config);

		let texture_bind_group_layout = device.create_bind_group_layout(
			&wgpu::BindGroupLayoutDescriptor { 
				label: Some("texture_binding_group_layout"),
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Texture {
							sample_type: wgpu::TextureSampleType::Float { filterable: true },
							view_dimension: wgpu::TextureViewDimension::D2,
							multisampled: false
						},
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
						count: None,
					},
				],
			}
		);

		let camera = Camera {
			// position the camera one unit up and 2 units back
			// +z is out of the screen
			eye: (0.0, 1.0, 2.0).into(),
			// have it look at the origin
			target: (0.0, 0.0, 0.0).into(),
			// which way is "up"
			up: cgmath::Vector3::unit_y(),
			aspect: config.width as f32 / config.height as f32,
			fovy: 45.0,
			znear: 0.1,
			zfar: 100.0,
		};

		let mut camera_uniform = CameraUniform::new();
		camera_uniform.update_view_proj(&camera);

		let camera_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: Some("Camera Buffer"),
				contents: bytemuck::cast_slice(&[camera_uniform]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			}
		);

		let camera_bind_group_layout = device.create_bind_group_layout(
			&wgpu::BindGroupLayoutDescriptor {
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStages::VERTEX,
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
		);

		let camera_bind_group = device.create_bind_group(
			&wgpu::BindGroupDescriptor {
				layout: &camera_bind_group_layout,
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: camera_buffer.as_entire_binding(),
					}
				],
				label: Some("camera_bind_group"),
			}
		);

		let camera_controller = CameraController::new(0.6);

		const NUM_INSTANCES_PER_ROW: u32 = 10;
		const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);
		const SPACE_BETWEEN: f32 = 3.0;

		let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
			(0..NUM_INSTANCES_PER_ROW).map(move |x| {
				let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
				let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
				
				let position = cgmath::Vector3 { x, y: 0.0, z};// - INSTANCE_DISPLACEMENT;
				
				let rotation = if position.is_zero() {
					cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(),cgmath::Deg(0.0))
				} else {
					cgmath::Quaternion::from_axis_angle( position.normalize(),cgmath::Deg(45.0))
				};

				Instance {
					position, rotation,
				}
			})
		}).collect::<Vec<_>>();

		let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
		let instance_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: Some("Instance Buffer"),
				contents: bytemuck::cast_slice(&instance_data),
				usage: wgpu::BufferUsages::VERTEX,
			}
		);

		let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let bg_color = wgpu::Color {
			r: 0.005,
			g: 0.005,
			b: 0.005,
			a: 1.0,
		};
        
		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Shader"),
			source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
		});

		let render_pipeline_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[
					&texture_bind_group_layout,
					&camera_bind_group_layout,
				],
				push_constant_ranges: &[],
			});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",					// vertex shader entrypoint
				buffers: &[
					model::ModelVertex::desc(),
					InstanceRaw::desc(),
				],							// vertex types?
			},
			fragment: Some(wgpu::FragmentState {		//optional?
				module: &shader,
				entry_point: "fs_main",					// fragment shader entrypoint
				targets:								// what color outputs to use
					&[Some(wgpu::ColorTargetState {
						format: config.format,
						blend: Some(wgpu::BlendState::REPLACE),		// just replace old pixel data
					 	write_mask: wgpu::ColorWrites::ALL,			// write to all color-channels
					})],
			}),
			primitive: wgpu::PrimitiveState {						// how to interpret vertices
				topology: wgpu::PrimitiveTopology::TriangleList,	// every three vertices = 1 triangle
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,					// if arranged counterclockwise triangle is facing forwards
				cull_mode: Some(wgpu::Face::Back),					// exclude triangles facing backwards
				unclipped_depth: false,
				polygon_mode: wgpu::PolygonMode::Fill,
				conservative: false,
			},
			depth_stencil: Some(wgpu::DepthStencilState {
				format: texture::Texture::DEPTH_FORMAT,
				depth_write_enabled: true,
				depth_compare: wgpu::CompareFunction::Less,
				stencil: wgpu::StencilState::default(),
				bias: wgpu::DepthBiasState::default(),
			}),
			multisample: wgpu::MultisampleState { 
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
		});

		let obj_model =
			resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
				.await
				.unwrap();

        //return
        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            bg_color,
			render_pipeline,
			camera,
			camera_uniform,
			camera_buffer,
			camera_bind_group,
			camera_controller,
			instances,
			instance_buffer,
			depth_texture,
			obj_model,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
    
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
        	self.size = new_size;
        	self.config.width = new_size.width;
        	self.config.height = new_size.height;
        	self.surface.configure(&self.device, &self.config);
			self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
		self.camera_controller.process_events(event);
		match &event {
			WindowEvent::KeyboardInput {
				input: 
					KeyboardInput {
						state: ElementState::Pressed,
						virtual_keycode: Some(VirtualKeyCode::Space),
						..
					},
					..
			} => {
				println!("Spacebar pressed.");
				true
			}
			_ => false
		}
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
		self.camera_uniform.update_view_proj(&self.camera);
		self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        	label: Some("Render Encoder"),
        });
        
        {
        	let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        		label: Some("Render Pass"),
        		color_attachments: &[
					Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(self.bg_color),
							store: true,
						},
        			})
				],
        		depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
					view: &self.depth_texture.view,
					depth_ops: Some(wgpu::Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: true,
					}),
					stencil_ops: None,
				}),
        	});

			render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
}

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

	let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
	    match event {
	    	
	        Event::WindowEvent {
	            ref event,
	            window_id,
	        } if window_id == state.window().id() => {
	        	if !state.input(event) {
		        	match event {
			            WindowEvent::CloseRequested 
			            | WindowEvent::KeyboardInput {
			                input:
			                    KeyboardInput {
			                        state: ElementState::Pressed,
			                        virtual_keycode: Some(VirtualKeyCode::Escape),
			                        ..
			                    },
			                ..
			            } => *control_flow = ControlFlow::Exit,
			            WindowEvent::Resized(physical_size) => {
			        		state.resize(*physical_size);
			        	}
			        	WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
			        		// new_inner_size is &&mut so we have to dereference it twice
			                state.resize(**new_inner_size);
			        	}
			            _ => {}
			        }
			  	}
			}
			Event::RedrawRequested(window_id) if window_id == state.window().id() => {
	    		state.update();
	    		match state.render() {
	    			Ok(_) => {}
	    			// Reconfigure the surface if lost
	    			Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
	    			// The system is out of memory, we should probably quit
	    			Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
	    			// All other errors (Outdated, Timeout) should be resolved by the next frame
	    			Err(e) => eprintln!("{:?}",e),
	    		}
	    	}
	    	Event::MainEventsCleared => {
	    		// RedrawRequested will only trigger once, unless we manually
            	// request it.
            	state.window().request_redraw();
	    	}
	        _ => {}
    	}
    });
    
}
