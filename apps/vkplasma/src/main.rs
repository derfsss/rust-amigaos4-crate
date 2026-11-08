//! vkplasma: an animated plasma drawn with Vulkan into an Intuition
//! window, through vulkan.library, the amigaos4 crate and ash.
//!
//!   vkplasma [FRAMES=n] [MAILBOX] [IMMEDIATE]   (FRAMES/K/N,MAILBOX/S,IMMEDIATE/S)
//!
//! A swapchain on a VK_AMIGA_surface; each frame one triangle covers the
//! window and the fragment shader draws the plasma from the time (a push
//! constant). The window can be resized (the swapchain is made again);
//! q, Esc, the close gadget or CTRL-C end it, or FRAMES n after n frames. The
//! frame rate is printed every 2 seconds and at the end. Present mode
//! FIFO (one frame per vertical blank) unless MAILBOX or IMMEDIATE is
//! given and the surface offers it; p cycles FIFO, MAILBOX and IMMEDIATE
//! while it runs (the swapchain is made again in the new mode). MAILBOX
//! and IMMEDIATE do not wait for the vertical blank: the frame rate is
//! the GPU's.
//!
//! A std (clib4) program: RADV opens the card only for a process with a C
//! library context.

use std::cell::Cell;
use std::time::Instant;

use amigaos4::intuition::{idcmp, Event, PubScreen, Window, WindowOptions};
use amigaos4::vulkan::{self, AmigaSurface, AMIGA_SURFACE_EXTENSION_NAME, KHR_SURFACE_EXTENSION_NAME};
use ash::{khr, vk};

/// The Vulkan loader and driver run on the caller's stack.
#[used]
static STACK: [u8; 15] = *b"$STACK:1048576\0";

static VERT_SPV: &[u8] = include_bytes!("../shaders/plasma.vert.spv");
static FRAG_SPV: &[u8] = include_bytes!("../shaders/plasma.frag.spv");

/// Frames the CPU may record ahead of the GPU.
const FRAMES_IN_FLIGHT: usize = 2;

/// SPIR-V words in host order from a file written little-endian (glslc).
fn spirv(bytes: &[u8]) -> Vec<u32> {
    let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    assert_eq!(words.first(), Some(&0x0723_0203), "not SPIR-V");
    words
}

fn fail(what: &str, r: impl std::fmt::Display) -> i32 {
    println!("vkplasma: {}: {}", what, r);
    20
}

struct Options {
    frames: Option<u64>,
    mode: vk::PresentModeKHR,
}

const TEMPLATE: &str = "FRAMES/K/N,MAILBOX/S,IMMEDIATE/S";

/// The command line, read by AmigaDOS's ReadArgs (the crate's dos::args).
fn options() -> Result<Options, String> {
    let a = amigaos4::dos::args::Args::from_command_line(TEMPLATE).map_err(|e| match e {
        amigaos4::error::AmigaError::DosError(c) => format!("{} (template {})", amigaos4::dos::fault(c), TEMPLATE),
        other => format!("{} (template {})", other, TEMPLATE),
    })?;
    let mode = if a.switch("IMMEDIATE").unwrap_or(false) {
        vk::PresentModeKHR::IMMEDIATE
    } else if a.switch("MAILBOX").unwrap_or(false) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    };
    let frames = a.number("FRAMES").ok().flatten().map(|n| n.max(0) as u64);
    Ok(Options { frames, mode })
}

/// The swapchain and what hangs off its images.
struct Chain {
    swapchain: vk::SwapchainKHR,
    extent: vk::Extent2D,
    views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    /// One per image: signalled when that image's rendering is done.
    rendered: Vec<vk::Semaphore>,
}

struct Gpu {
    instance: ash::Instance,
    surface_fn: khr::surface::Instance,
    surface: vk::SurfaceKHR,
    pd: vk::PhysicalDevice,
    device: ash::Device,
    swap_fn: khr::swapchain::Device,
    queue: vk::Queue,
    format: vk::SurfaceFormatKHR,
    /// The present mode in use (p cycles it) and those the surface offers.
    mode: Cell<vk::PresentModeKHR>,
    modes: Vec<vk::PresentModeKHR>,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    pool: vk::CommandPool,
    cmds: Vec<vk::CommandBuffer>,
    acquired: Vec<vk::Semaphore>,
    in_flight: Vec<vk::Fence>,
}

impl Gpu {
    /// Make (or remake, passing the old one) the swapchain for the
    /// surface's current size. None while the window has no area.
    unsafe fn make_chain(&self, window: &Window<'_>, old: vk::SwapchainKHR) -> Result<Option<Chain>, vk::Result> {
        let caps = self.surface_fn.get_physical_device_surface_capabilities(self.pd, self.surface)?;
        let extent = if caps.current_extent.width != u32::MAX {
            caps.current_extent
        } else {
            let (x0, y0, x1, y1) = window.inner();
            vk::Extent2D { width: (x1 - x0 + 1).max(0) as u32, height: (y1 - y0 + 1).max(0) as u32 }
        };
        if extent.width == 0 || extent.height == 0 {
            return Ok(None);
        }
        let mut count = caps.min_image_count + 1;
        if caps.max_image_count > 0 {
            count = count.min(caps.max_image_count);
        }
        let ci = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(count)
            .image_format(self.format.format)
            .image_color_space(self.format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.mode.get())
            .clipped(true)
            .old_swapchain(old);
        let swapchain = self.swap_fn.create_swapchain(&ci, None)?;
        let images = self.swap_fn.get_swapchain_images(swapchain)?;
        let mut chain = Chain { swapchain, extent, views: Vec::new(), framebuffers: Vec::new(), rendered: Vec::new() };
        for &img in &images {
            let vi = vk::ImageViewCreateInfo::default()
                .image(img)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(self.format.format)
                .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
            let view = self.device.create_image_view(&vi, None)?;
            chain.views.push(view);
            let attachments = [view];
            let fi = vk::FramebufferCreateInfo::default().render_pass(self.render_pass).attachments(&attachments).width(extent.width).height(extent.height).layers(1);
            chain.framebuffers.push(self.device.create_framebuffer(&fi, None)?);
            chain.rendered.push(self.device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?);
        }
        Ok(Some(chain))
    }

    /// Everything of `chain` but the swapchain itself.
    unsafe fn drop_chain_parts(&self, chain: &Chain) {
        for &f in &chain.framebuffers {
            self.device.destroy_framebuffer(f, None);
        }
        for &v in &chain.views {
            self.device.destroy_image_view(v, None);
        }
        for &s in &chain.rendered {
            self.device.destroy_semaphore(s, None);
        }
    }

    unsafe fn record(&self, cmd: vk::CommandBuffer, chain: &Chain, image: usize, time: f32) -> Result<(), vk::Result> {
        let d = &self.device;
        d.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())?;
        d.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT))?;
        let clear = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
        let area = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: chain.extent };
        let rp = vk::RenderPassBeginInfo::default().render_pass(self.render_pass).framebuffer(chain.framebuffers[image]).render_area(area).clear_values(&clear);
        d.cmd_begin_render_pass(cmd, &rp, vk::SubpassContents::INLINE);
        d.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: chain.extent.width as f32, height: chain.extent.height as f32, min_depth: 0.0, max_depth: 1.0 };
        d.cmd_set_viewport(cmd, 0, &[viewport]);
        d.cmd_set_scissor(cmd, 0, &[area]);
        // Two floats in host order; the driver converts what the GPU reads.
        let push = [time, chain.extent.width as f32 / chain.extent.height as f32];
        let bytes: Vec<u8> = push.iter().flat_map(|f| f.to_ne_bytes()).collect();
        d.cmd_push_constants(cmd, self.layout, vk::ShaderStageFlags::FRAGMENT, 0, &bytes);
        d.cmd_draw(cmd, 3, 1, 0, 0);
        d.cmd_end_render_pass(cmd);
        d.end_command_buffer(cmd)
    }
}

/// Instance, surface, device, render pass, pipeline and sync objects.
unsafe fn make_gpu(entry: &ash::Entry, window: &Window<'_>, want_mode: vk::PresentModeKHR) -> Result<Gpu, String> {
    let exts = [KHR_SURFACE_EXTENSION_NAME.as_ptr(), AMIGA_SURFACE_EXTENSION_NAME.as_ptr()];
    let app = vk::ApplicationInfo::default().application_name(c"vkplasma").engine_name(c"amigaos4 crate").api_version(vk::API_VERSION_1_1);
    let instance = entry.create_instance(&vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&exts), None).map_err(|r| format!("vkCreateInstance: {}", r))?;
    let surface = AmigaSurface::new(entry, &instance).map_err(|e| format!("VK_AMIGA_surface: {}", e))?.create(window).map_err(|r| format!("vkCreateAmigaSurfaceAMIGA: {}", r))?;
    let surface_fn = khr::surface::Instance::new(entry, &instance);

    // The first device with a queue family that draws and presents here.
    let mut chosen = None;
    for pd in instance.enumerate_physical_devices().map_err(|r| format!("vkEnumeratePhysicalDevices: {}", r))? {
        for (i, q) in instance.get_physical_device_queue_family_properties(pd).iter().enumerate() {
            let present = surface_fn.get_physical_device_surface_support(pd, i as u32, surface).unwrap_or(false);
            if q.queue_flags.contains(vk::QueueFlags::GRAPHICS) && present {
                chosen = Some((pd, i as u32));
                break;
            }
        }
        if chosen.is_some() {
            break;
        }
    }
    let Some((pd, family)) = chosen else {
        return Err("no device draws and presents to this window (is radv_vk.library installed and VK_DRIVER_FILES set?)".into());
    };
    let props = instance.get_physical_device_properties(pd);
    println!("device: {}", props.device_name_as_c_str().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());

    let prio = [1.0f32];
    let qci = [vk::DeviceQueueCreateInfo::default().queue_family_index(family).queue_priorities(&prio)];
    let dexts = [khr::swapchain::NAME.as_ptr()];
    let device = instance.create_device(pd, &vk::DeviceCreateInfo::default().queue_create_infos(&qci).enabled_extension_names(&dexts), None).map_err(|r| format!("vkCreateDevice: {}", r))?;
    let queue = device.get_device_queue(family, 0);
    let swap_fn = khr::swapchain::Device::new(&instance, &device);

    let formats = surface_fn.get_physical_device_surface_formats(pd, surface).map_err(|r| format!("surface formats: {}", r))?;
    let format = formats.iter().copied().find(|f| f.format == vk::Format::B8G8R8A8_UNORM).or(formats.first().copied()).ok_or("the surface lists no format")?;
    let modes = surface_fn.get_physical_device_surface_present_modes(pd, surface).unwrap_or_default();
    let mode = if modes.contains(&want_mode) { want_mode } else { vk::PresentModeKHR::FIFO };

    let attachment = [vk::AttachmentDescription::default()
        .format(format.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];
    let colour_ref = [vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL }];
    let subpass = [vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).color_attachments(&colour_ref)];
    // The colour write waits for the acquire (signalled at that stage).
    let dependency = [vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];
    let render_pass = device.create_render_pass(&vk::RenderPassCreateInfo::default().attachments(&attachment).subpasses(&subpass).dependencies(&dependency), None).map_err(|r| format!("vkCreateRenderPass: {}", r))?;

    let vert_code = spirv(VERT_SPV);
    let frag_code = spirv(FRAG_SPV);
    let vert = device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&vert_code), None).map_err(|r| format!("vertex shader: {}", r))?;
    let frag = device.create_shader_module(&vk::ShaderModuleCreateInfo::default().code(&frag_code), None).map_err(|r| format!("fragment shader: {}", r))?;
    let push = [vk::PushConstantRange { stage_flags: vk::ShaderStageFlags::FRAGMENT, offset: 0, size: 8 }];
    let layout = device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&push), None).map_err(|r| format!("pipeline layout: {}", r))?;
    let stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vert).name(c"main"),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(frag).name(c"main"),
    ];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let viewport = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    let raster = vk::PipelineRasterizationStateCreateInfo::default().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).front_face(vk::FrontFace::COUNTER_CLOCKWISE).line_width(1.0);
    let multisample = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let blend_attachment = [vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA)];
    let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachment);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let pci = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&assembly)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&blend)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(), &[pci], None).map_err(|(_, r)| format!("vkCreateGraphicsPipelines: {}", r))?[0];
    device.destroy_shader_module(vert, None);
    device.destroy_shader_module(frag, None);

    let pool = device.create_command_pool(&vk::CommandPoolCreateInfo::default().queue_family_index(family).flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER), None).map_err(|r| format!("command pool: {}", r))?;
    let cmds = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default().command_pool(pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(FRAMES_IN_FLIGHT as u32)).map_err(|r| format!("command buffers: {}", r))?;
    let mut acquired = Vec::new();
    let mut in_flight = Vec::new();
    for _ in 0..FRAMES_IN_FLIGHT {
        acquired.push(device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).map_err(|r| format!("semaphore: {}", r))?);
        in_flight.push(device.create_fence(&vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED), None).map_err(|r| format!("fence: {}", r))?);
    }
    Ok(Gpu { instance, surface_fn, surface, pd, device, swap_fn, queue, format, mode: Cell::new(mode), modes, render_pass, layout, pipeline, pool, cmds, acquired, in_flight })
}

fn main() {
    std::process::exit(run_main());
}

fn run_main() -> i32 {
    let opts = match options() {
        Ok(o) => o,
        Err(e) => return fail("arguments", e),
    };
    let entry = match vulkan::entry() {
        Ok(e) => e,
        Err(e) => return fail("vulkan.library", e),
    };
    let scr = match PubScreen::default_screen() {
        Ok(s) => s,
        Err(e) => return fail("public screen", e),
    };
    let mut win = match Window::open(&scr, &WindowOptions {
        title: "vkplasma",
        left: 80,
        top: 60,
        inner_width: 640,
        inner_height: 400,
        idcmp: idcmp::CLOSEWINDOW | idcmp::NEWSIZE | idcmp::VANILLAKEY,
        size_gadget: true,
        ..Default::default()
    }) {
        Ok(w) => w,
        Err(e) => return fail("window", e),
    };
    // SAFETY (all Vulkan calls below): ash's commands with arguments the
    // Vulkan specification allows; every object is destroyed, in reverse
    // order, before the window closes.
    let gpu = match unsafe { make_gpu(&entry, &win, opts.mode) } {
        Ok(g) => g,
        Err(e) => return fail("setup", e),
    };
    let rc = unsafe { run(&gpu, &mut win, opts.frames) };
    unsafe {
        let _ = gpu.device.device_wait_idle();
        for k in 0..FRAMES_IN_FLIGHT {
            gpu.device.destroy_semaphore(gpu.acquired[k], None);
            gpu.device.destroy_fence(gpu.in_flight[k], None);
        }
        gpu.device.destroy_command_pool(gpu.pool, None);
        gpu.device.destroy_pipeline(gpu.pipeline, None);
        gpu.device.destroy_pipeline_layout(gpu.layout, None);
        gpu.device.destroy_render_pass(gpu.render_pass, None);
        gpu.device.destroy_device(None);
        // The surface goes before the window.
        gpu.surface_fn.destroy_surface(gpu.surface, None);
        gpu.instance.destroy_instance(None);
    }
    drop(win);
    rc
}

fn mode_name(m: vk::PresentModeKHR) -> &'static str {
    match m {
        vk::PresentModeKHR::MAILBOX => "MAILBOX",
        vk::PresentModeKHR::IMMEDIATE => "IMMEDIATE",
        vk::PresentModeKHR::FIFO => "FIFO",
        _ => "other",
    }
}

/// The render loop; the swapchain is made and remade here and destroyed
/// before returning.
unsafe fn run(gpu: &Gpu, win: &mut Window<'_>, frames: Option<u64>) -> i32 {
    let mut chain: Option<Chain> = None;
    let mut stale = true;
    let start = Instant::now();
    let mut mark = (Instant::now(), 0u64);
    let mut done = 0u64;
    let mut frame = 0usize;
    let mut rebuilds = 0u32;
    let mut rc = 0;
    'run: loop {
        while let Some(m) = win.next() {
            match m.event {
                Event::CloseWindow => break 'run,
                Event::Key(b'q' | b'Q' | 27) => break 'run,
                Event::Key(b'p' | b'P') => {
                    // The next of FIFO, MAILBOX, IMMEDIATE the surface offers.
                    let order = [vk::PresentModeKHR::FIFO, vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE];
                    let at = order.iter().position(|&m| m == gpu.mode.get()).unwrap_or(0);
                    if let Some(next) = (1..=order.len()).map(|k| order[(at + k) % order.len()]).find(|m| gpu.modes.contains(m)) {
                        gpu.mode.set(next);
                        println!("present mode: {}", mode_name(next));
                        stale = true;
                        mark = (Instant::now(), done);
                    }
                }
                Event::NewSize => stale = true,
                _ => {}
            }
        }
        if frames.is_some_and(|n| done >= n) {
            break;
        }
        if amigaos4::signal::take(amigaos4::sys::SIGBREAKF_CTRL_C) != 0 {
            println!("CTRL-C");
            break;
        }
        if stale {
            let _ = gpu.device.device_wait_idle();
            let old = chain.take();
            let made = gpu.make_chain(win, old.as_ref().map(|c| c.swapchain).unwrap_or(vk::SwapchainKHR::null()));
            if let Some(o) = &old {
                gpu.drop_chain_parts(o);
                gpu.swap_fn.destroy_swapchain(o.swapchain, None);
            }
            match made {
                Ok(Some(c)) => {
                    println!("swapchain: {}x{}, {} images, format {}, {}", c.extent.width, c.extent.height, c.views.len(), gpu.format.format.as_raw(), mode_name(gpu.mode.get()));
                    chain = Some(c);
                    rebuilds += 1;
                    stale = false;
                }
                Ok(None) => {
                    // No area to draw into: wait for the next size.
                    amigaos4::signal::wait(win.signal_mask());
                    continue;
                }
                Err(r) => {
                    rc = fail("swapchain", r);
                    break;
                }
            }
        }
        let Some(c) = chain.as_ref() else { break };
        let fence = gpu.in_flight[frame];
        if let Err(r) = gpu.device.wait_for_fences(&[fence], true, u64::MAX) {
            rc = fail("vkWaitForFences", r);
            break;
        }
        let image = match gpu.swap_fn.acquire_next_image(c.swapchain, u64::MAX, gpu.acquired[frame], vk::Fence::null()) {
            Ok((i, _suboptimal)) => i as usize,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                stale = true;
                continue;
            }
            Err(r) => {
                rc = fail("vkAcquireNextImageKHR", r);
                break;
            }
        };
        let _ = gpu.device.reset_fences(&[fence]);
        let cmd = gpu.cmds[frame];
        if let Err(r) = gpu.record(cmd, c, image, start.elapsed().as_secs_f32()) {
            rc = fail("recording", r);
            break;
        }
        let waits = [gpu.acquired[frame]];
        let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let cmds = [cmd];
        let signals = [c.rendered[image]];
        let submit = vk::SubmitInfo::default().wait_semaphores(&waits).wait_dst_stage_mask(&stages).command_buffers(&cmds).signal_semaphores(&signals);
        if let Err(r) = gpu.device.queue_submit(gpu.queue, &[submit], fence) {
            rc = fail("vkQueueSubmit", r);
            break;
        }
        let chains = [c.swapchain];
        let images = [image as u32];
        let present = vk::PresentInfoKHR::default().wait_semaphores(&signals).swapchains(&chains).image_indices(&images);
        match gpu.swap_fn.queue_present(gpu.queue, &present) {
            Ok(false) => {}
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => stale = true,
            Err(r) => {
                rc = fail("vkQueuePresentKHR", r);
                break;
            }
        }
        done += 1;
        frame = (frame + 1) % FRAMES_IN_FLIGHT;
        let since = mark.0.elapsed().as_secs_f64();
        if since >= 2.0 {
            let fps = (done - mark.1) as f64 / since;
            println!("{} frames, {:.1} fps", done, fps);
            let _ = win.set_title(&format!("vkplasma: {:.1} fps ({})", fps, mode_name(gpu.mode.get())));
            mark = (Instant::now(), done);
        }
    }
    let _ = gpu.device.device_wait_idle();
    if let Some(c) = chain.take() {
        gpu.drop_chain_parts(&c);
        gpu.swap_fn.destroy_swapchain(c.swapchain, None);
    }
    let secs = start.elapsed().as_secs_f64();
    println!("{} frames in {:.2} s: {:.1} fps; swapchain made {} time(s)", done, secs, done as f64 / secs.max(1e-9), rebuilds);
    rc
}
