//mod model;
use winit::dpi::PhysicalSize;

pub fn create_surface_format(surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    surface_caps.formats.iter()
        .copied()
        .filter(|f| f.describe().srgb)
        .next()
        .unwrap_or(surface_caps.formats[0])
}

pub fn create_surface_config(size: &PhysicalSize<u32>, surface_caps: &wgpu::SurfaceCapabilities) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: create_surface_format(surface_caps),
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    }
}

pub fn create_render_pipeline_layout(device: &wgpu::Device, texture_bind_group_layout: &wgpu::BindGroupLayout, camera_bind_group_layout: &wgpu::BindGroupLayout, light_bind_group_layout: &wgpu::BindGroupLayout) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            texture_bind_group_layout,
            camera_bind_group_layout,
            light_bind_group_layout,
        ],
        push_constant_ranges: &[],
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    label: Option<&str>
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: label,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",					// vertex shader entrypoint
            buffers: vertex_layouts,							// vertex types?
        },
        fragment: Some(wgpu::FragmentState {		//optional?
            module: &shader,
            entry_point: "fs_main",					// fragment shader entrypoint
            targets: &[
                Some(wgpu::ColorTargetState {			// what color outputs to use 
                    format: color_format,
                    blend: Some(wgpu::BlendState{
                        alpha: wgpu::BlendComponent::REPLACE,    // just replace old pixel data
                        color: wgpu::BlendComponent::REPLACE,
                    }),		
                    write_mask: wgpu::ColorWrites::ALL,			// write to all color-channels
                }),
            ],
        }),
        primitive: wgpu::PrimitiveState {						// how to interpret vertices
            topology: wgpu::PrimitiveTopology::TriangleList,	// every three vertices = 1 triangle
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,					// if arranged counterclockwise triangle is facing forwards
            cull_mode: Some(wgpu::Face::Back),					// exclude triangles facing backwards
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
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
    })
}