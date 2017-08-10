// Copyright 2015 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate gfx;
extern crate winit;
extern crate cgmath;

use gfx::{Adapter, CommandQueue, Device, FrameSync, GraphicsPoolExt, Surface, SwapChain};
use gfx::traits::DeviceExt;

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    vertex Vertex {
        pos: [f32; 3] = "a_Pos",
        color: [f32; 3] = "a_Color",
    }

    constant Locals {
        transform: [[f32; 4]; 4] = "u_Transform",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4]; 4]> = "u_Transform",
        locals: gfx::ConstantBuffer<Locals> = "Locals",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

const CUBE_VERTS: [Vertex; 8] = [
    // top
    Vertex {
        pos: [-1., -1., 1.],
        color: [0., 1., 0.],
    },
    Vertex {
        pos: [1., -1., 1.],
        color: [0., 1., 0.],
    },
    Vertex {
        pos: [1., 1., 1.],
        color: [0., 1., 0.],
    },
    Vertex {
        pos: [-1., 1., 1.],
        color: [0., 1., 0.],
    },
    // bottom
    Vertex {
        pos: [-1., -1., -1.],
        color: [0., 0., 1.],
    },
    Vertex {
        pos: [1., -1., -1.],
        color: [0., 0., 1.],
    },
    Vertex {
        pos: [1., 1., -1.],
        color: [0., 0., 1.],
    },
    Vertex {
        pos: [-1., 1., -1.],
        color: [0., 0., 1.],
    },
];

const CUBE_INDICES: [[u16; 6]; 6] = [
    // top
    [0, 1, 2, 2, 3, 0],
    // bottom
    [4, 5, 6, 6, 7, 4],
    // right
    [0, 1, 4, 4, 5, 1],
    // left
    [1, 2, 5, 5, 6, 2],
    // bottom
    [2, 3, 6, 6, 7, 3],
    // right
    [3, 0, 7, 7, 4, 0],
];

const CLEAR_COLOR: [f32; 4] = [0.1, 0.2, 0.3, 1.0];

mod backend {
    extern crate gfx_device_gl;
    extern crate gfx_window_glutin;
    extern crate glutin;

    use gfx::WindowExt;
    use winit;

    pub type WinSurface = gfx_window_glutin::Surface;
    pub type WinAdapter = gfx_device_gl::Adapter;
    pub type Dimensions = (u32, u32);

    // TODO: Factor this out into struct
    pub fn get_surface_and_adapters(
        events_loop: &winit::EventsLoop,
    ) -> (Dimensions, WinSurface, Vec<WinAdapter>) {
        // Create window
        let wb = glutin::WindowBuilder::new()
            .with_title("Triangle example".to_string())
            .with_dimensions(1024, 768);
        let gl_builder = glutin::ContextBuilder::new().with_vsync(true);
        let window = glutin::GlWindow::new(wb, gl_builder, events_loop).expect("Can't get window");
        let dim = window.get_inner_size_points().expect(
            "Can't get window dimensions",
        );

        // Acquire surface and adapters
        let out = gfx_window_glutin::Window::new(window).get_surface_and_adapters();

        (dim, out.0, out.1)
    }
}

pub fn main() {
    use self::backend::get_surface_and_adapters;

    fn mk_transform(dim: (u32, u32), angle: f32) -> [[f32; 4]; 4] {
        use cgmath::{Deg, Matrix3, Matrix4, Point3, Quaternion, Rotation3, Vector3, perspective};

        let rot: Matrix3<f32> = Quaternion::from_angle_z(Deg(angle)).into();

        let default_view = Matrix4::look_at(
            Point3::new(0f32, 0., 0.) + rot * Vector3::new(1.5f32, -5.0, 3.0),
            Point3::new(0f32, 0.0, 0.0),
            Vector3::unit_z(),
        );

        let proj = perspective(Deg(45.), dim.0 as f32 / dim.1 as f32, 1.0, 10.0);

        (proj * default_view).into()
    }

    // Create window
    let mut events_loop = winit::EventsLoop::new();
    let (mut dim, mut surface, adapters) = get_surface_and_adapters(&events_loop);

    // Open gpu (device and queues)
    let gfx::Gpu {
        mut device,
        mut graphics_queues,
        ..
    } = adapters.get(0).expect("No adapters found").open_with(
        |family, ty| {
            (
                (ty.supports_graphics() && surface.supports_queue(family)) as u32,
                gfx::QueueType::Graphics,
            )
        },
    );
    let mut graphics_queue = graphics_queues.pop().expect(
        "Unable to find a graphics queue.",
    );

    // Create swapchain
    let config = gfx::SwapchainConfig::new()
        .with_color::<ColorFormat>()
        .with_depth_stencil::<DepthFormat>();
    let mut swap_chain = surface.build_swapchain(config, &graphics_queue);
    let views = swap_chain
        .get_backbuffers()
        .into_iter()
        .map(|&(ref color, ref ds)| {
            use gfx::texture::{DepthStencilDesc, DepthStencilFlags, RenderDesc};
            use gfx::handle::{DepthStencilView, RenderTargetView};
            use gfx::memory::Typed;
            use gfx::format::Formatted;

            let color_desc = RenderDesc {
                channel: ColorFormat::get_format().1,
                level: 0,
                layer: None,
            };
            let rtv = device
                .view_texture_as_render_target_raw(color, color_desc)
                .expect("Can't get view texture");

            let ds_desc = DepthStencilDesc {
                level: 0,
                layer: None,
                flags: DepthStencilFlags::empty(),
            };
            let dsv = device
                .view_texture_as_depth_stencil_raw(ds.as_ref().expect("No depth"), ds_desc)
                .expect("Can't get depth stencil from texture");
            let out: (RenderTargetView<_, ColorFormat>,
                      DepthStencilView<_, DepthFormat>) = (Typed::new(rtv), Typed::new(dsv));

            out
        })
        .collect::<Vec<_>>();

    let pso = device
        .create_pipeline_simple(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/shader/triangle_150.glslv"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/shader/triangle_150.glslf"
            )),
            pipe::new(),
        )
        .unwrap();

    let flat_indices: [u16; 6 * 6] = unsafe { std::mem::transmute(CUBE_INDICES) };
    let (vertex_buffer, slice) =
        device.create_vertex_buffer_with_slice(&CUBE_VERTS, flat_indices.as_ref());
    let mut graphics_pool = graphics_queue.create_graphics_pool(1);
    let frame_semaphore = device.create_semaphore();
    let draw_semaphore = device.create_semaphore();
    let frame_fence = device.create_fence(false);

    let mut angle = 45.;
    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        transform: mk_transform(dim, angle),
        locals: device.create_constant_buffer(1),
        out_color: views[0].0.clone(),
        out_depth: views[0].1.clone(),
    };

    // main loop
    let mut running = true;
    while running {
        // fetch events
        events_loop.poll_events(|event| if let winit::Event::WindowEvent {
            event, ..
        } = event
        {
            match event {
                winit::WindowEvent::Closed => running = false,
                winit::WindowEvent::KeyboardInput {
                    input: winit::KeyboardInput {
                        virtual_keycode: Some(winit::VirtualKeyCode::Escape), ..
                    },
                    ..
                } => return,
                winit::WindowEvent::Resized(width, height) => {
                    dim = (width, height);
                }
                _ => (),
            }
        });

        angle += 1.;
        angle %= 360.;
        data.transform = mk_transform(dim, angle);

        // Get next frame
        let frame = swap_chain.acquire_frame(FrameSync::Semaphore(&frame_semaphore));
        data.out_color = views[frame.id()].0.clone();

        // draw a frame
        // wait for frame -> draw -> signal -> present
        {
            let mut encoder = graphics_pool.acquire_graphics_encoder();

            let locals = Locals { transform: data.transform };
            encoder.update_constant_buffer(&data.locals, &locals);

            encoder.clear(&data.out_color, CLEAR_COLOR);
            encoder.clear_depth(&data.out_depth, 1.0);

            encoder.draw(&slice, &pso, &data);
            encoder
                .synced_flush(
                    &mut graphics_queue,
                    &[&frame_semaphore],
                    &[&draw_semaphore],
                    Some(&frame_fence),
                )
                .expect("Could not flush encoder");
        }

        swap_chain.present(&mut graphics_queue, &[&draw_semaphore]);
        device.wait_for_fences(&[&frame_fence], gfx::WaitFor::All, 1_000_000);
        graphics_queue.cleanup();
        graphics_pool.reset();
    }
}
