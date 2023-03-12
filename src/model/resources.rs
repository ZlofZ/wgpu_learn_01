use std::io::{Cursor, BufReader};

use anyhow::Ok;
use wgpu::util::DeviceExt;

use crate::model;


pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader, 
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            //println!("{}", mat_text);
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    ).await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        println!("{:?}", m);
        println!("Material name: {}, texture file: {}, normal: {}", m.name, m.diffuse_texture, m.normal_texture);
        let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        let mut normal_texture_file_name = m.normal_texture;
        if normal_texture_file_name.eq("") {
            normal_texture_file_name = String::from("cube-normal.png");
        }else if normal_texture_file_name.contains(" ") {
            while normal_texture_file_name.contains(" ") {
                let space_loc = normal_texture_file_name.find(" ").unwrap();
                normal_texture_file_name.replace_range(..space_loc+1, "");
            }
            println!("Trimmed normal filename: {}", normal_texture_file_name);
        }
        let normal_texture = load_texture(&normal_texture_file_name, device, queue).await?;
        
        materials.push(model::Material::new(
            device,
            &m.name,
            diffuse_texture,
            normal_texture,
            layout,
        ));
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3 )
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [
                        m.mesh.texcoords[i * 2],
                        m.mesh.texcoords[i * 2 + 1],
                    ],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                }).collect::<Vec<_>>();

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", file_name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", file_name)),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                model::Mesh {
                    name: file_name.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
        }).collect::<Vec<_>>();

        Ok(model::Model { meshes, materials })
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    println!("filename: {}", file_name);
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let txt = std::fs::read_to_string(path)?;

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let data = std::fs::read(path)?;
    
    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device:&wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<model::texture::Texture> {
    let data = load_binary(file_name).await?;
    model::texture::Texture::from_bytes(device, queue, &data, file_name)
}