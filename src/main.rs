use std::ffi::{CStr, CString, c_char, c_void};

use ash::{
    Device, Entry, Instance,
    khr::surface,
    vk::{
        API_VERSION_1_3, ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo,
        InstanceCreateInfo, KHR_SWAPCHAIN_NAME, PhysicalDevice, PhysicalDeviceFeatures,
        PhysicalDeviceProperties2, PhysicalDeviceVulkan12Features, PhysicalDeviceVulkan13Features,
        Queue, QueueFamilyProperties2, QueueFlags, SurfaceKHR,
    },
};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::EventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
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
    physical_device: Option<PhysicalDevice>,
    device: Option<Device>,
    queue_family: Option<usize>,
    queue: Option<Queue>,
    surface: Option<SurfaceKHR>,
    surface_instance: Option<surface::Instance>,
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
                .map_err(error_to_string)?,
        );
        let app_info = ApplicationInfo::default()
            .application_name(CStr::from_bytes_with_nul(b"vkt-sw\0").map_err(error_to_string)?)
            .api_version(API_VERSION_1_3);
        let instance_extensions = ash_window::enumerate_required_extensions(
            self.window
                .as_ref()
                .unwrap()
                .display_handle()
                .map_err(error_to_string)?
                .as_raw(),
        )
        .map_err(error_to_string)?;

        let instance_create_info = InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(instance_extensions);
        let entry = unsafe { Entry::load().map_err(error_to_string)? };
        self.instance = Some(unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(error_to_string)?
        });

        let physical_devices = unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .enumerate_physical_devices()
                .map_err(error_to_string)
        }?;

        self.physical_device = Some(physical_devices[0]);

        let mut physical_device_properties = PhysicalDeviceProperties2::default();
        unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .get_physical_device_properties2(
                    *self.physical_device.as_ref().unwrap(),
                    &mut physical_device_properties,
                )
        };

        let device_name = unsafe {
            CStr::from_ptr(
                physical_device_properties
                    .properties
                    .device_name
                    .as_mut_ptr(),
            )
        }
        .to_str();

        match device_name {
            Ok(dn) => info!("device name: {dn}"),
            Err(e) => return Err(error_to_string(e)),
        }
        let queue_family_properties_len = 0;
        unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .get_physical_device_queue_family_properties2_len(
                    *self.physical_device.as_ref().unwrap(),
                )
        };

        let mut queue_family_properties: Vec<QueueFamilyProperties2> = Vec::new();
        queue_family_properties.resize(
            queue_family_properties_len,
            QueueFamilyProperties2::default(),
        );
        unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .get_physical_device_queue_family_properties2(
                    *self.physical_device.as_ref().unwrap(),
                    queue_family_properties.as_mut_slice(),
                )
        };

        self.queue_family = Some(0);
        for (i, qfp) in queue_family_properties.iter().enumerate() {
            if qfp
                .queue_family_properties
                .queue_flags
                .contains(QueueFlags::GRAPHICS)
            {
                self.queue_family = Some(i);
                break;
            }
        }

        self.surface = Some(unsafe {
            ash_window::create_surface(
                &entry,
                self.instance.as_ref().unwrap(),
                self.window
                    .as_ref()
                    .unwrap()
                    .display_handle()
                    .map_err(error_to_string)?
                    .as_raw(),
                self.window
                    .as_ref()
                    .unwrap()
                    .window_handle()
                    .map_err(error_to_string)?
                    .as_raw(),
                None,
            )
            .map_err(error_to_string)
        }?);
        trace!("Created surface");
        self.surface_instance = Some(surface::Instance::new(
            &entry,
            self.instance.as_ref().unwrap(),
        ));
        trace!("Created surface instance");

        if !unsafe {
            self.surface_instance
                .as_ref()
                .unwrap()
                .get_physical_device_surface_support(
                    *self.physical_device.as_ref().unwrap(),
                    *self.queue_family.as_ref().unwrap() as u32,
                    *self.surface.as_ref().unwrap(),
                )
                .map_err(error_to_string)
        }? {
            return Err("Queue Family does not support Present!".to_owned());
        }

        trace!(
            "Found queue family index: {}",
            self.queue_family.as_ref().unwrap()
        );

        let queue_create_info = DeviceQueueCreateInfo::default()
            .queue_family_index(*self.queue_family.as_ref().unwrap() as u32)
            .queue_priorities(&[1.0]);

        let device_extensions: &[*const i8] = &[KHR_SWAPCHAIN_NAME.as_ptr()];
        let device_features_12 = PhysicalDeviceVulkan12Features::default()
            .descriptor_indexing(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .descriptor_binding_variable_descriptor_count(true)
            .runtime_descriptor_array(true)
            .buffer_device_address(true);
        let mut device_features_13 = PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);
        device_features_13.p_next =
            &device_features_12 as *const PhysicalDeviceVulkan12Features as *mut _;
        let device_features_10 = PhysicalDeviceFeatures::default().sampler_anisotropy(true);

        let binding = [queue_create_info.clone()];
        let device_create_info = DeviceCreateInfo::default()
            .queue_create_infos(&binding)
            .enabled_extension_names(device_extensions)
            .enabled_features(&device_features_10)
            .push_next(&mut device_features_13);

        self.device = Some(unsafe {
            self.instance
                .as_ref()
                .unwrap()
                .create_device(
                    *self.physical_device.as_ref().unwrap(),
                    &device_create_info,
                    None,
                )
                .map_err(error_to_string)?
        });

        trace!("Created device");

        self.queue = Some(unsafe {
            self.device
                .as_ref()
                .unwrap()
                .get_device_queue(*self.queue_family.as_ref().unwrap() as u32, 0)
        });

        trace!("Created queue");

        self.is_init = true;
        Ok(())
    }

    pub fn is_init(&self) -> bool {
        self.is_init
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        trace!("[resume] resumed!");
        if !self.is_init() {
            match self.init(event_loop) {
                Ok(_) => trace!("[resume] Successfully initialized app!"),
                Err(e) => {
                    error!("[resume] Failed to initialize app: {e}");
                    event_loop.exit();
                }
            }
        }
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

fn error_to_string<T>(e: T) -> String
where
    T: std::fmt::Debug,
{
    format!("{e:?}")
}
