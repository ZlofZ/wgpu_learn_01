use winit::{window::Window, event::{WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}};
use cgmath::prelude::*;
use wgpu::util::DeviceExt;

use crate::{camera::{Camera, CameraUniform, CameraController, self}, model::{DrawModel,Model, instance::{self, Instance}, texture, resources}, state};

pub mod renderer;


pub(crate) struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    bg_color: wgpu::Color,
	render_pipeline: wgpu::RenderPipeline,
	camera: Camera,
	camera_uniform: CameraUniform,
	camera_buffer: wgpu::Buffer,
	camera_bind_group: wgpu::BindGroup,
	camera_controller: CameraController,
	instances: Vec<instance::Instance>,
	instance_buffer: wgpu::Buffer,
	depth_texture: texture::Texture,
	obj_model: Model,
}

impl State {
	
    // Creating some of the wgpu types requires async code
    pub async fn new(window: Window) -> Self {
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
        //let surface_format = state::renderer::create_surface_format(&surface_caps);

        let config = renderer::create_surface_config(&size, &surface_caps);
        surface.configure(&device, &config);

		let texture_bind_group_layout = texture::create_texture_bind_group_layout(&device);

		let camera = camera::create_camera(&config);

		let mut camera_uniform = CameraUniform::new();
		camera_uniform.update_view_proj(&camera);
		let camera_buffer = camera::create_camera_buffer(&device, &camera_uniform);
		let camera_bind_group_layout = camera::create_camera_bind_group_layout(&device);
		let camera_bind_group = camera::create_camera_bind_group(&device, &camera_bind_group_layout, &camera_buffer);
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

				instance::Instance {
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
			source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
		});

		let render_pipeline_layout = state::renderer::create_render_pipeline_layout(&device, &texture_bind_group_layout, &camera_bind_group_layout);

		let render_pipeline = state::renderer::create_render_pipeline(&device, &render_pipeline_layout, &shader, &config);
		 
		let obj_model =
			resources::load_model("SceneTrackmania.obj", &device, &queue, &texture_bind_group_layout)
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
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
        	self.size = new_size;
        	self.config.width = new_size.width;
        	self.config.height = new_size.height;
        	self.surface.configure(&self.device, &self.config);
			self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
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

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
		self.camera_uniform.update_view_proj(&self.camera);
		self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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