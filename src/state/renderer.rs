//mod model;
use model::Vertex;
use winit::dpi::PhysicalSize;

use crate::{model::{instance, texture, self}};

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

pub fn create_render_pipeline_layout(device: &wgpu::Device, texture_bind_group_layout: &wgpu::BindGroupLayout, camera_bind_group_layout: &wgpu::BindGroupLayout) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[
            &texture_bind_group_layout,
            &camera_bind_group_layout,
        ],
        push_constant_ranges: &[],
    })
}

pub fn create_render_pipeline(device: &wgpu::Device, render_pipeline_layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule, config: &wgpu::SurfaceConfiguration) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",					// vertex shader entrypoint
            buffers: &[
                model::ModelVertex::desc(),
                instance::InstanceRaw::desc(),
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
    })
}