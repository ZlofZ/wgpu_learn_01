//use wgpu::RenderPipeline;
use wgpu::util::DeviceExt;
use winit::window::Window;

// lib.rs

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 3],
	color: [f32; 3],
}

impl Vertex {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x3,
				}
			]
		}
	}
}

const VERTICES: &[Vertex] = &[

	Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
	Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
	Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
	Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
	Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E

    Vertex { position: [0.2, 0.2, 0.0], color: [0.5, 0.0, 0.5] }, // 5
    Vertex { position: [-0.2, 0.2, 0.0], color: [0.5, 0.0, 0.5] }, // 6
    Vertex { position: [0.2, -0.2, 0.0], color: [0.5, 0.0, 0.5] }, // 7
    Vertex { position: [-0.2, -0.2, 0.0], color: [0.5, 0.0, 0.5] }, // 8

    Vertex { position: [0.3, 0.3, 0.0], color: [0.15, 0.0, 0.15] }, // 9
    Vertex { position: [0.3, -0.1, 0.0], color: [0.15, 0.0, 0.15] }, // 10
	
    Vertex { position: [-0.1, 0.3, 0.0], color: [0.15, 0.0, 0.15] }, // 11
	

];

const INDICES: &[u16] = &[

	5, 6, 7,
	6, 8, 7,
	5, 7, 9,
	9, 7, 10,
	11, 6, 5,
	11, 5, 9,
];
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    bg_color: wgpu::Color,
	render_pipeline: wgpu::RenderPipeline,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	num_indices: u32,
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
				bind_group_layouts: &[],
				push_constant_ranges: &[],
			});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",					// vertex shader entrypoint
				buffers: &[
					Vertex::desc(),
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
				depth_stencil: None,
				multisample: wgpu::MultisampleState { 
					count: 1,
					mask: !0,
					alpha_to_coverage_enabled: false,
				},
				multiview: None,
			});

		let vertex_buffer = device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(VERTICES),
				usage: wgpu::BufferUsages::VERTEX,
			}
		);
        
		let index_buffer =  device.create_buffer_init(
			&wgpu::util::BufferInitDescriptor { 
				label: Some("Index Buffer IND1"),
				contents: bytemuck::cast_slice(INDICES),
				usage: wgpu::BufferUsages::INDEX,
			}
		);

		let num_indices = INDICES.len() as u32;

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
			vertex_buffer,
			index_buffer,
			num_indices,
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
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
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
        		depth_stencil_attachment: None,
        	});

			
			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
			render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

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
