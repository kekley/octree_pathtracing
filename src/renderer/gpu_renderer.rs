use crate::{
    colors::colors::PixelColor,
    gpu_structs::{
        gpu_camera::UniformData,
        gpu_material::GPUMaterial,
        gpu_octree::{octree_to_gpu_data, GPUOctreeNode, GPUOctreeUniform},
        gpu_quad::GPUQuad,
    },
    scene::scene::Scene,
    textures::material,
};
use std::{fs, num::NonZero, slice, sync::Arc, time::Instant};

use super::{
    camera::Camera,
    renderer_trait::{FrameInFlight, FrameInFlightPoll, RenderingBackend},
    tile_renderer::{RendererMode, RendererStatus},
};
use bytemuck::Zeroable;
use eframe::{
    egui::TextureHandle,
    wgpu::{
        self,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
        BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor, ComputePipeline,
        ComputePipelineDescriptor, Device, Extent3d, MaintainBase, Origin3d, PipelineLayout,
        PipelineLayoutDescriptor, Queue, SamplerDescriptor, ShaderModule, ShaderModuleDescriptor,
        ShaderSource, ShaderStages, StorageTextureAccess, SubmissionIndex, TexelCopyTextureInfo,
        Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewDimension,
    },
};
use glam::Vec3A;
use log::info;
pub fn create_render_data(
    device: &Device,
    render_bind_group_layout: &BindGroupLayout,
    uniform_data: &UniformData,
    index_stack: &[u32; 24],
    time_stack: &[f32; 24],
) -> RenderData {
    let context_uniform = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("uniform"),
        contents: bytemuck::cast_slice(slice::from_ref(uniform_data)),
        usage: BufferUsages::UNIFORM,
    });
    let index_stack = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("index stack"),
        contents: bytemuck::cast_slice(index_stack),
        usage: BufferUsages::STORAGE,
    });
    let time_stack = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("time stack"),
        contents: bytemuck::cast_slice(time_stack),
        usage: BufferUsages::STORAGE,
    });

    let render_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Render bind group"),
        layout: &render_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: context_uniform.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: index_stack.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: time_stack.as_entire_binding(),
            },
        ],
    });
    RenderData {
        render_uniform: context_uniform,
        render_bind_group: render_bind_group,
        index_stack,
        time_stack,
    }
}

pub struct RenderData {
    render_uniform: Buffer,
    index_stack: Buffer,
    time_stack: Buffer,
    render_bind_group: BindGroup,
}

pub struct SVOPipeline {
    pub octree_uniform_buffer: Buffer,
    pub octree_buffer: Buffer,
    pub octree_bind_group: BindGroup,
    pub textures: Vec<Texture>,
    pub texture_views: Vec<TextureView>,
    pub material_buffer: Buffer,
    pub quad_buffer: Buffer,
    pub shader: ShaderModule,
    pub octree_bind_group_layout: BindGroupLayout,
    pub render_bind_group_layout: BindGroupLayout,
    pub pipeline_layout: PipelineLayout,
    pub compute_pipeline: ComputePipeline,
    pub output_texture: Texture,
    pub texture_view: TextureView,
}

impl SVOPipeline {
    pub fn destroy_buffers(&mut self) {
        self.octree_buffer.destroy();
        self.quad_buffer.destroy();
        self.material_buffer.destroy();
        self.octree_uniform_buffer.destroy();
        self.textures.iter_mut().for_each(|t| {
            t.destroy();
        });
    }
    pub fn change_resolution(&mut self, device: &Device, resolution: (u32, u32)) {
        //gonna have to do more here when the beam optimization is implemented
        self.output_texture.destroy();
        let new_texture = device.create_texture(&TextureDescriptor {
            label: Some("Output Texture"),
            size: Extent3d {
                width: resolution.0,
                height: resolution.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });
        let texture_view = new_texture.create_view(&TextureViewDescriptor {
            label: Some("Output Texture View"),
            format: Some(TextureFormat::Rgba8Unorm),
            dimension: Some(TextureViewDimension::D2),
            usage: Some(TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        });
        self.output_texture = new_texture;
        self.texture_view = texture_view;
    }
}

pub struct GPURenderer {
    render_size: (u32, u32),
    scene: Option<Arc<parking_lot::RwLock<Scene>>>,
    camera: Camera,
    device: Device,
    queue: Queue,
    mode: RendererMode,
    status: RendererStatus,
    render_data: Option<RenderData>,
    pipeline: Option<SVOPipeline>,
}
pub struct GPUFrameInFlight {
    device: Device,
    submission_index: SubmissionIndex,
    texture: TextureHandle,
}

impl FrameInFlight for GPUFrameInFlight {
    fn poll(self: Box<GPUFrameInFlight>) -> FrameInFlightPoll {
        let result = &self.device.poll(MaintainBase::Poll);
        match result {
            wgpu::MaintainResult::SubmissionQueueEmpty => FrameInFlightPoll::Ready(self.texture),
            wgpu::MaintainResult::Ok => FrameInFlightPoll::NotReady(self),
        }
    }
    fn wait_for(self: Box<GPUFrameInFlight>) -> Result<TextureHandle, TextureHandle> {
        let a = self.device.poll(wgpu::MaintainBase::WaitForSubmissionIndex(
            self.submission_index,
        ));
        match a {
            wgpu::MaintainResult::SubmissionQueueEmpty => Err(self.texture),
            wgpu::MaintainResult::Ok => Ok(self.texture),
        }
    }
}

impl GPURenderer {
    pub fn new(device: &Device, queue: &Queue, render_size: (u32, u32)) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),
            mode: RendererMode::Preview,
            pipeline: None,
            scene: None,
            camera: Camera::look_at(Vec3A::ZERO, Vec3A::Z, Vec3A::Y, 70.0f32.to_radians()),
            render_data: None,
            render_size,
            status: RendererStatus::Stopped,
        }
    }
    pub fn create_pipeline(device: &Device, queue: &Queue, scene: &Scene) -> SVOPipeline {
        let octree = &scene.octree;

        let materials = &scene.materials;
        let quads = &scene.quads;

        let (octree_uniform, octant_data) = octree_to_gpu_data(&scene.octree);

        info!(
            "GPU Data:\n Octree memory: {}MB, Materials Memory: {}MB, Quad Memory: {}MB",
            (octant_data.len() * 4 * 8) as f32 / 1000000.0,
            (materials.len() * size_of::<GPUMaterial>()) as f32 / 1000000.0,
            (quads.len() * size_of::<GPUQuad>()) as f32 / 1000000.0
        );
        let shader_code = fs::read_to_string("./assets/shaders/svo.wgsl").unwrap();
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });

        let ((textures, texture_views), materials): (
            (Vec<Texture>, Vec<TextureView>),
            Vec<GPUMaterial>,
        ) = materials
            .iter()
            .enumerate()
            .map(|(i, material)| match &material.texture {
                crate::textures::texture::Texture::Color(u8_color) => {
                    let string = format!("{:?}", u8_color);
                    let label = Some(string.as_str());
                    let data: [u8; 4] = [u8_color.r(), u8_color.g(), u8_color.b(), u8_color.a()];
                    let descriptor = TextureDescriptor {
                        label,
                        size: Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8Uint,
                        usage: TextureUsages::TEXTURE_BINDING,
                        view_formats: &[TextureFormat::Rgba8Uint],
                    };
                    let texture = device.create_texture_with_data(
                        queue,
                        &descriptor,
                        eframe::wgpu::util::TextureDataOrder::LayerMajor,
                        &data,
                    );
                    let texture_view = texture.create_view(&TextureViewDescriptor {
                        label,
                        format: None,
                        dimension: None,
                        usage: None,
                        aspect: eframe::wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: Some(1),
                    });

                    let material = GPUMaterial::zeroed();
                    ((texture, texture_view), material)
                }
                crate::textures::texture::Texture::Image(rtwimage) => {
                    let string = format!("{:p}", &rtwimage);
                    let label = Some(string.as_str());

                    let data = &rtwimage.raw_data;
                    let descriptor = TextureDescriptor {
                        label,
                        size: Extent3d {
                            width: rtwimage.image_width,
                            height: rtwimage.image_height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8Uint,
                        usage: TextureUsages::TEXTURE_BINDING,
                        view_formats: &[TextureFormat::Rgba8Uint],
                    };
                    let texture = device.create_texture_with_data(
                        queue,
                        &descriptor,
                        eframe::wgpu::util::TextureDataOrder::LayerMajor,
                        data,
                    );
                    let texture_view = texture.create_view(&TextureViewDescriptor {
                        label,
                        format: None,
                        dimension: None,
                        usage: None,
                        aspect: eframe::wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: Some(1),
                    });
                    let material = GPUMaterial::zeroed();
                    ((texture, texture_view), material)
                }
            })
            .collect();

        let octree_uniform = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Octree Uniform Data"),
            contents: bytemuck::cast_slice(slice::from_ref(&octree_uniform)),
            usage: BufferUsages::UNIFORM,
        });

        let quads: Vec<_> = quads.iter().map(|quad| GPUQuad::from(quad)).collect();

        let material_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("materials"),
            contents: bytemuck::cast_slice(&materials),
            usage: BufferUsages::STORAGE,
        });
        let quad_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("quads"),
            contents: bytemuck::cast_slice(&quads),
            usage: BufferUsages::STORAGE,
        });
        let octree_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("octants"),
            contents: bytemuck::cast_slice(&octant_data),
            usage: BufferUsages::STORAGE,
        });

        let output_texture = device.create_texture(&TextureDescriptor {
            label: Some("Output Texture"),
            size: Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        let output_texture_view = output_texture.create_view(&TextureViewDescriptor {
            usage: Some(TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC),
            label: Some("Ouput Texture View"),
            format: Some(TextureFormat::Rgba8Unorm),
            dimension: Some(TextureViewDimension::D2),
            aspect: eframe::wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Render bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: NonZero::<u32>::new(24),
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Nearest Sampler"),
            address_mode_u: eframe::wgpu::AddressMode::Repeat,
            address_mode_v: eframe::wgpu::AddressMode::Repeat,
            address_mode_w: eframe::wgpu::AddressMode::Repeat,
            mag_filter: eframe::wgpu::FilterMode::Nearest,
            min_filter: eframe::wgpu::FilterMode::Nearest,
            mipmap_filter: eframe::wgpu::FilterMode::Nearest,
            lod_min_clamp: 1.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        let octree_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Octree Data Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    //octants
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    //quads
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    //Materials
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    //textures
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: eframe::wgpu::TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: Some(((texture_views.len()) as u32).try_into().unwrap()),
                    },
                    //sampler
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(eframe::wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let view_array: Vec<_> = texture_views.iter().map(|view| view).collect();

        let octree_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Octree bind group"),
            layout: &octree_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: octree_uniform.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: octree_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: quad_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: material_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureViewArray(&view_array),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&output_texture_view),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&render_bind_group_layout, &octree_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        return SVOPipeline {
            shader: module,
            pipeline_layout,
            compute_pipeline: pipeline,
            texture_view: output_texture_view,
            octree_buffer,
            octree_bind_group,
            output_texture,
            textures,
            texture_views,
            material_buffer,
            quad_buffer,
            octree_bind_group_layout,
            render_bind_group_layout,
            octree_uniform_buffer: octree_uniform,
        };
    }
}

impl RenderingBackend for GPURenderer {
    fn render_frame(
        &self,
        eframe: &eframe::Frame,
        texture: TextureHandle,
    ) -> Result<Box<dyn super::renderer_trait::FrameInFlight>, TextureHandle> {
        let svo_pipeline = match &self.pipeline {
            Some(pipeline) => pipeline,
            None => return Err(texture),
        };
        let scene = match &self.scene {
            Some(scene) => scene.read(),
            None => return Err(texture),
        };
        let render_state = eframe.wgpu_render_state().unwrap().renderer.read();
        let inner_texture = match render_state.texture(&texture.id()) {
            Some(texture) => texture,
            None => return Err(texture),
        };
        let ray = self.camera.get_ray(0.0, 0.0);
        let aspect_ratio = self.render_size.0 as f32 / self.render_size.1 as f32;
        let (traversal_start_index, scale, index_stack, time_stack) = todo!();
        let device = &self.device;
        let queue = &self.queue;
        let d_factor = 1.0 / (self.camera.fov / 2.0).tan();
        let view_up_ortho = (self.camera.up
            - self.camera.up.dot(self.camera.direction) * self.camera.direction)
            .normalize();
        let view_right = self.camera.direction.cross(view_up_ortho);

        let uniform_data = UniformData {
            camera_scaled_view_dir: (self.camera.direction * d_factor).to_array(),
            traversal_start_idx: traversal_start_index,
            camera_scaled_view_right: (aspect_ratio * view_right).to_array(),
            scale: scale,
            camera_view_up_ortho: view_up_ortho.to_array(),
            inv_image_size_x: 1.0 / self.render_size.0 as f32,
            camera_world_position: self.camera.eye.to_array(),
            inv_image_size_y: 1.0 / self.render_size.1 as f32,
        };
        let render_data = create_render_data(
            device,
            &svo_pipeline.render_bind_group_layout,
            &uniform_data,
            &index_stack,
            &time_stack,
        );
        let pipeline = &svo_pipeline.compute_pipeline;
        let octree_bind_group = &svo_pipeline.octree_bind_group;
        let render_bind_group = &render_data.render_bind_group;
        let mut command_encoder =
            device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(1, octree_bind_group, &[]);
            compute_pass.set_bind_group(0, render_bind_group, &[]);
            compute_pass.dispatch_workgroups(1280, 720, 1);
        }

        command_encoder.copy_texture_to_texture(
            TexelCopyTextureInfo {
                texture: &svo_pipeline.output_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: eframe::wgpu::TextureAspect::All,
            },
            TexelCopyTextureInfo {
                texture: &inner_texture.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: eframe::wgpu::TextureAspect::All,
            },
            Extent3d {
                width: 1280,
                height: 720,
                depth_or_array_layers: 1,
            },
        );

        let index = queue.submit(Some(command_encoder.finish()));
        let start = Instant::now();
        device.poll(MaintainBase::Wait);
        let time = Instant::now().duration_since(start);
        info!("Took {time:?} to render on GPU");
        Ok(Box::new(GPUFrameInFlight {
            device: self.device.clone(),
            submission_index: index,
            texture,
        }))
    }

    fn update_scene(&mut self, ctx: &eframe::egui::Context) {
        self.camera.move_with_keyboard_input(ctx);
        self.camera.rotate(ctx);
    }

    fn set_scene(&mut self, scene: &std::sync::Arc<parking_lot::RwLock<Scene>>) {
        self.scene = Some(scene.clone());
        let scene = scene.read();
        if self.pipeline.is_some() {}
        self.pipeline = Some(Self::create_pipeline(&self.device, &self.queue, &scene));
    }

    fn get_mode(&self) -> RendererMode {
        self.mode
    }

    fn get_status(&self) -> RendererStatus {
        self.status
    }

    fn get_resolution(&self) -> (u32, u32) {
        (self.render_size.0, self.render_size.1)
    }

    fn set_resolution(&mut self, resolution: (u32, u32)) {
        self.render_size = resolution;
    }

    fn set_mode(&mut self, mode: RendererMode) {
        self.mode = mode;
    }

    fn which_backend(&self) -> crate::settings::RendererBackendSetting {
        crate::settings::RendererBackendSetting::GPU
    }

    fn get_camera(&self) -> &Camera {
        &self.camera
    }

    fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }
}
