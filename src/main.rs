use std::ffi::CStr;

use ash::{
    Entry, Instance,
    vk::{API_VERSION_1_3, ApplicationInfo, InstanceCreateInfo},
};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::EventLoop,
    raw_window_handle::HasDisplayHandle,
    window::{Window, WindowAttributes},
};

use spdlog::prelude::*;

fn main() -> std::result::Result<(), EventLoopError> {
    spdlog::default_logger().set_level_filter(LevelFilter::All);
    let event_loop = EventLoop::new().expect("Failed to create event loop!");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)
}

#[derive(Default)]
struct App {
    is_init: bool,
    instance: Option<Instance>,
    window: Option<Window>,
}

impl App {
    pub fn init(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> std::result::Result<(), String> {
        self.window = Some(
            event_loop
                .create_window(WindowAttributes::default().with_title("vkt-sw"))
                .map_err(error_to_str)?,
        );
        let app_info = ApplicationInfo::default()
            .application_name(CStr::from_bytes_with_nul(b"vkt-sw\0").map_err(error_to_str)?)
            .api_version(API_VERSION_1_3);
        let instance_extensions = ash_window::enumerate_required_extensions(
            self.window
                .as_ref()
                .unwrap()
                .display_handle()
                .map_err(error_to_str)?
                .as_raw(),
        )
        .map_err(error_to_str)?;

        let instance_create_info = InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(instance_extensions);
        let entry = unsafe { Entry::load().map_err(error_to_str)? };
        self.instance = Some(unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(error_to_str)?
        });
        Ok(())
    }

    pub fn is_init(&self) -> bool {
        self.is_init
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        info!("[resume] resumed!");
        if !self.is_init() {
            match self.init(event_loop) {
                Ok(_) => info!("[resume] Successfully initialized app!"),
                Err(e) => error!("[resume] Failed to initialize app: {e}"),
            }
        }
        todo!()
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
}

fn error_to_str<T>(e: T) -> String
where
    T: std::fmt::Debug,
{
    format!("{e:?}")
}
