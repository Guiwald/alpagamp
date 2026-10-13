//! Accès au GPU (wgpu) et présentation dans une fenêtre : des calques texturés (la visualisation, puis des images
//! RGBA dessinées par le CPU : cadre de skin, messages) composés dans la surface de la fenêtre.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Le GPU partagé par toutes les fenêtres.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    sampler_linear: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    shader: wgpu::ShaderModule,
    pipelines: std::cell::RefCell<Vec<(wgpu::TextureFormat, bool, wgpu::RenderPipeline)>>,
}

/// Instance wgpu : Vulkan, Metal ou DX12 (la variable `WGPU_BACKEND` peut l'imposer).
pub fn instance() -> wgpu::Instance {
    let mut d = wgpu::InstanceDescriptor::new_without_display_handle();
    d.backends = wgpu::Backends::PRIMARY;
    wgpu::Instance::new(d.with_env())
}

/// Crée une surface pour une fenêtre.
///
/// # Safety
/// La fenêtre doit vivre plus longtemps que la surface.
pub unsafe fn surface<W: HasWindowHandle + HasDisplayHandle>(instance: &wgpu::Instance, window: &W) -> Result<wgpu::Surface<'static>, String> {
    let target = unsafe { wgpu::SurfaceTargetUnsafe::from_display_and_window(window, window) }.map_err(|e| e.to_string())?;
    unsafe { instance.create_surface_unsafe(target) }.map_err(|e| e.to_string())
}

const SHADER: &str = r#"
struct Quad { dst: vec4<f32>, src: vec4<f32> };
@group(0) @binding(0) var<uniform> q: Quad;
@group(0) @binding(1) var t: texture_2d<f32>;
@group(0) @binding(2) var s: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> VOut {
    let c = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    var o: VOut;
    o.pos = vec4<f32>(mix(q.dst.x, q.dst.z, c.x), mix(q.dst.y, q.dst.w, c.y), 0.0, 1.0);
    o.uv = vec2<f32>(mix(q.src.x, q.src.z, c.x), mix(q.src.y, q.src.w, c.y));
    return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
    return textureSample(t, s, v.uv);
}
"#;

impl Gpu {
    /// Choisit un adaptateur (compatible avec `surface` si donnée) et ouvre le périphérique.
    pub fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface>, low_power: bool) -> Result<Gpu, String> {
        // L'adaptateur préféré d'abord, puis les autres (un pilote peut refuser le périphérique : on essaie le
        // suivant, par exemple Vulkan après DirectX 12).
        let mut candidates = Vec::new();
        if let Ok(a) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: if low_power { wgpu::PowerPreference::LowPower } else { wgpu::PowerPreference::HighPerformance },
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })) {
            candidates.push(a);
        }
        for a in pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all())) {
            let info = a.get_info();
            let known = candidates.iter().any(|c: &wgpu::Adapter| {
                let ci = c.get_info();
                ci.backend == info.backend && ci.name == info.name
            });
            if !known && surface.is_none_or(|s| a.is_surface_supported(s)) {
                candidates.push(a);
            }
        }
        // Les GPU logiciels en dernier.
        candidates.sort_by_key(|a| a.get_info().device_type == wgpu::DeviceType::Cpu);
        let mut last = String::from("aucun GPU utilisable");
        let mut found = None;
        for adapter in candidates {
            let info = adapter.get_info();
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("alpagamp"),
                required_limits: adapter.limits(),
                ..Default::default()
            })) {
                Ok((device, queue)) => {
                    log::info!("GPU : {} ({:?}, {})", info.name, info.backend, info.driver);
                    found = Some((adapter, device, queue));
                    break;
                }
                Err(e) => {
                    log::warn!("GPU {} ({:?}) refusé : {e}", info.name, info.backend);
                    last = format!("GPU refusé : {e}");
                }
            }
        }
        let (adapter, device, queue) = found.ok_or(last)?;
        device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| log::error!("wgpu : {e}")));
        let sampler = |f: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: f,
                min_filter: f,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            })
        };
        let sampler_linear = sampler(wgpu::FilterMode::Linear);
        let sampler_nearest = sampler(wgpu::FilterMode::Nearest);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("calque"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("calques"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        Ok(Gpu { instance, adapter, device, queue, sampler_linear, sampler_nearest, layout, shader, pipelines: Default::default() })
    }

    fn pipeline(&self, format: wgpu::TextureFormat, blend: bool) -> wgpu::RenderPipeline {
        let mut cache = self.pipelines.borrow_mut();
        if let Some((_, _, p)) = cache.iter().find(|(f, b, _)| *f == format && *b == blend) {
            return p.clone();
        }
        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("calques"),
            bind_group_layouts: &[Some(&self.layout)],
            immediate_size: 0,
        });
        let p = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("calques"),
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &self.shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        cache.push((format, blend, p.clone()));
        p
    }
}

/// Une image RGBA (dessinée par le CPU) gardée sur le GPU.
pub struct Layer {
    texture: wgpu::Texture,
    uniforms: wgpu::Buffer,
    bind: wgpu::BindGroup,
    pub size: (u32, u32),
    linear: bool,
}

/// Tranche d'octets d'une image `0xAARRGGBB` : en mémoire (petit-boutiste), c'est du BGRA.
fn argb_bytes(px: &[u32]) -> &[u8] {
    // SAFETY : u32 n'a pas de bits de remplissage ; la longueur est multipliée par 4 ; l'alignement de u8 est 1.
    unsafe { std::slice::from_raw_parts(px.as_ptr() as *const u8, std::mem::size_of_val(px)) }
}

impl Layer {
    /// Calque d'une texture existante (la visualisation).
    pub fn from_view(gpu: &Gpu, texture: &wgpu::Texture, linear: bool) -> Layer {
        let view = texture.create_view(&Default::default());
        let uniforms = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("calque"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("calque"),
            layout: &gpu.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(if linear { &gpu.sampler_linear } else { &gpu.sampler_nearest }),
                },
            ],
        });
        Layer { texture: texture.clone(), uniforms, bind, size: (texture.width(), texture.height()), linear }
    }

    /// Calque vide de cette taille, à remplir avec [`Layer::upload_argb`].
    pub fn new(gpu: &Gpu, w: u32, h: u32, linear: bool) -> Layer {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("calque"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Les pixels du CPU (`0xAARRGGBB`) sont du BGRA en mémoire : envoyés tels quels.
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Layer::from_view(gpu, &texture, linear)
    }

    /// Envoie des pixels `0xAARRGGBB` (le canal alpha compte quand le calque est mélangé).
    pub fn upload_argb(&mut self, gpu: &Gpu, w: u32, h: u32, argb: &[u32]) {
        if (w, h) != self.size || self.texture.format() != wgpu::TextureFormat::Bgra8Unorm {
            *self = Layer::new(gpu, w, h, self.linear);
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            argb_bytes(&argb[..(w * h) as usize]),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
}

/// Rectangle en pixels de la cible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
}

/// La surface d'une fenêtre.
pub struct Presenter {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    view_format: wgpu::TextureFormat,
}

/// Un calque à dessiner : où, et s'il se mélange (alpha) à ce qui est dessous.
pub struct Draw<'a> {
    pub layer: &'a Layer,
    pub dst: Rect,
    pub blend: bool,
}

impl Presenter {
    pub fn new(gpu: &Gpu, surface: wgpu::Surface<'static>, w: u32, h: u32, vsync: bool) -> Result<Presenter, String> {
        let caps = surface.get_capabilities(&gpu.adapter);
        // Les pixels du CPU et de MilkDrop sont déjà « écran » : surface sans conversion sRGB.
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).or(caps.formats.first().copied()).ok_or("surface sans format")?;
        let view_format = format.remove_srgb_suffix();
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) { wgpu::CompositeAlphaMode::Opaque } else { caps.alpha_modes[0] };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: w.max(1),
            height: h.max(1),
            present_mode: if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync },
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: if view_format != format { vec![view_format] } else { vec![] },
        };
        surface.configure(&gpu.device, &config);
        Ok(Presenter { surface, config, view_format })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn resize(&mut self, gpu: &Gpu, w: u32, h: u32) {
        if (w.max(1), h.max(1)) != (self.config.width, self.config.height) {
            self.config.width = w.max(1);
            self.config.height = h.max(1);
            self.surface.configure(&gpu.device, &self.config);
        }
    }

    /// Dessine les calques dans l'ordre (fond noir), avec `before` pour enregistrer d'autres passes (le rendu de
    /// MilkDrop) dans le même encodeur. Renvoie `false` si l'image n'a pas pu être présentée.
    pub fn frame(&mut self, gpu: &Gpu, before: impl FnOnce(&mut wgpu::CommandEncoder), draws: &[Draw]) -> bool {
        let mut reconfigure = false;
        let tex = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => t,
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                reconfigure = true;
                t
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&gpu.device, &self.config);
                return false;
            }
            _ => return false,
        };
        let view = tex.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(self.view_format), ..Default::default() });
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        before(&mut enc);
        let (sw, sh) = (self.config.width as f32, self.config.height as f32);
        for d in draws {
            let x0 = d.dst.x / sw * 2.0 - 1.0;
            let x1 = (d.dst.x + d.dst.w) / sw * 2.0 - 1.0;
            let y0 = 1.0 - d.dst.y / sh * 2.0;
            let y1 = 1.0 - (d.dst.y + d.dst.h) / sh * 2.0;
            let data: [f32; 8] = [x0, y0, x1, y1, 0.0, 0.0, 1.0, 1.0];
            let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
            gpu.queue.write_buffer(&d.layer.uniforms, 0, &bytes);
        }
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composition"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for d in draws {
                pass.set_pipeline(&gpu.pipeline(self.view_format, d.blend));
                pass.set_bind_group(0, &d.layer.bind, &[]);
                pass.draw(0..4, 0..1);
            }
        }
        gpu.queue.submit(Some(enc.finish()));
        gpu.queue.present(tex);
        if reconfigure {
            self.surface.configure(&gpu.device, &self.config);
        }
        true
    }
}

/// Lit une texture `Rgba8Unorm` (lent : tests et captures seulement).
pub fn read_rgba(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u8> {
    let (w, h) = (texture.width(), texture.height());
    let row = (w * 4).div_ceil(256) * 256;
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lecture"),
        size: (row * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    gpu.queue.submit(Some(enc.finish()));
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    if let Ok(data) = slice.get_mapped_range() {
        for y in 0..h {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
    }
    buf.unmap();
    out
}
