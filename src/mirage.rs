mod device;
mod forward_renderer;
// mod mesh;
mod simple_pass;
mod simple_pass_object;
mod swap_chain;

use ash::{vk, Entry};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::borrow::Cow;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::os;
use std::rc::Rc;
use winit::event_loop::ControlFlow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const VALIDATION_LAYERS: &[&CStr] =
    &[unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") }];

#[cfg(all(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!("[{message_severity:?}:{message_types:?}] {message_id_name} ({message_id_number}):\n{message}");

    vk::FALSE
}

pub struct Mirage {
    #[allow(dead_code)]
    entry: Entry,
    event_loop: Option<EventLoop<()>>,
    window: Window,
    // instance: Arc<ash::Instance>,
    instance: Rc<ash::Instance>,
    debug_utils_loader: Option<ash::extensions::ext::DebugUtils>,
    debug_utils_messenger: Option<vk::DebugUtilsMessengerEXT>,
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
    device: ManuallyDrop<Rc<device::Device>>,
    swap_chain: ManuallyDrop<Rc<swap_chain::SwapChain>>,
    forward_renderer: ManuallyDrop<Rc<forward_renderer::ForwardRenderer>>,
    simple_pass: ManuallyDrop<simple_pass::SimplePass>,
}

impl Mirage {
    pub fn initialize() -> Self {
        let entry = Entry::linked();
        let (event_loop, window) = Self::init_window();
        let instance = Rc::new(Self::create_instance(&entry, &window));
        let (debug_utils_loader, debug_utils_messenger) =
            Self::setup_debug_utils(&entry, &instance);
        let (surface_loader, surface) = Self::create_surface(&entry, &instance, &window);
        let device = Rc::new(device::Device::new(
            Rc::clone(&instance),
            &surface_loader,
            surface,
        ));
        let swap_chain = Rc::new(swap_chain::SwapChain::new(
            &instance,
            &window,
            Rc::clone(&device),
            surface,
        ));
        let forward_renderer = Rc::new(forward_renderer::ForwardRenderer::new(
            &instance,
            Rc::clone(&device),
            Rc::clone(&swap_chain),
        ));
        let mut simple_pass =
            simple_pass::SimplePass::new(Rc::clone(&device), Rc::clone(&forward_renderer));
        simple_pass.add_object(simple_pass_object::SimplePassObject::new(&simple_pass));

        Self {
            entry,
            event_loop: Some(event_loop),
            window,
            debug_utils_loader,
            debug_utils_messenger,
            instance,
            surface_loader,
            surface,
            device: ManuallyDrop::new(device),
            swap_chain: ManuallyDrop::new(swap_chain),
            forward_renderer: ManuallyDrop::new(forward_renderer),
            simple_pass: ManuallyDrop::new(simple_pass),
        }
    }

    pub fn main_loop(&mut self) {
        let mut is_closed = false;
        self.event_loop
            .take()
            .unwrap()
            .run(move |event, elwt| {
                // println!("event_loop event: {event:?}");

                match event {
                    Event::WindowEvent { event, window_id } if self.window.id() == window_id => {
                        match event {
                            WindowEvent::CloseRequested => {
                                is_closed = true;
                                elwt.exit();
                                unsafe {
                                    self.device.device.device_wait_idle().unwrap();
                                }
                            },
                            WindowEvent::RedrawRequested => {
                                if is_closed {
                                    return;
                                }
                                self.forward_renderer.render(&self.simple_pass);
                            }
                            _ => (),
                        }
                    }
                    Event::AboutToWait => {
                        self.window.request_redraw();
                    }
                    _ => (),
                }
            })
            .unwrap();
    }

    fn init_window() -> (EventLoop<()>, Window) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let window = WindowBuilder::new()
            .with_title("Mirage")
            .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
            // .with_inner_size(winit::dpi::PhysicalSize::<u32>::from((WIDTH, HEIGHT)))
            .build(&event_loop)
            .unwrap();

        (event_loop, window)
    }

    fn create_instance(entry: &Entry, window: &Window) -> ash::Instance {
        if ENABLE_VALIDATION_LAYERS && !Self::check_validation_layers_support(&entry) {
            panic!("Validation layers requested, but not available!")
        }

        unsafe {
            let app_name = CStr::from_bytes_with_nul_unchecked(b"Mirage\0");

            let app_info = vk::ApplicationInfo::builder()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 0, 0))
                .build();

            let layer_names = VALIDATION_LAYERS
                .iter()
                .cloned()
                .map(|layer| layer.as_ptr())
                .collect::<Vec<_>>();

            let mut extension_names =
                ash_window::enumerate_required_extensions(window.raw_display_handle())
                    .unwrap()
                    .to_vec();

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                extension_names.push(vk::KhrPortabilityEnumerationFn::name().as_ptr());
                // required by *device* extension VK_KHR_portability_subset
                extension_names.push(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
            }

            if ENABLE_VALIDATION_LAYERS {
                extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
            }

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::default()
            };

            let mut debug_info = Self::build_debug_utils_messenger_create_info();
            let create_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&layer_names)
                .enabled_extension_names(&extension_names)
                .flags(create_flags)
                .push_next(&mut debug_info)
                .build();

            entry
                .create_instance(&create_info, None)
                .expect("Instance creation failed")
        }
    }

    fn check_validation_layers_support(entry: &Entry) -> bool {
        let supported_layers = entry
            .enumerate_instance_layer_properties()
            .unwrap()
            .iter()
            .map(|layer| unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) })
            .collect::<Vec<_>>();

        VALIDATION_LAYERS
            .iter()
            .all(|layer| supported_layers.contains(layer))
    }

    fn setup_debug_utils(
        entry: &Entry,
        instance: &ash::Instance,
    ) -> (
        Option<ash::extensions::ext::DebugUtils>,
        Option<vk::DebugUtilsMessengerEXT>,
    ) {
        if !ENABLE_VALIDATION_LAYERS {
            return (None, None);
        }

        unsafe {
            let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
            let debug_info = Self::build_debug_utils_messenger_create_info();
            let debug_utils_messenger = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .expect("failed to setup debug messenger!");

            (Some(debug_utils_loader), Some(debug_utils_messenger))
        }
    }

    fn build_debug_utils_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback))
            .build()
    }

    fn create_surface(
        entry: &Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> (ash::extensions::khr::Surface, vk::SurfaceKHR) {
        unsafe {
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);
            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .unwrap();

            (surface_loader, surface)
        }
    }
}

impl Drop for Mirage {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.simple_pass);
            ManuallyDrop::drop(&mut self.forward_renderer);
            ManuallyDrop::drop(&mut self.swap_chain);
            ManuallyDrop::drop(&mut self.device);

            self.surface_loader.destroy_surface(self.surface, None);
            if self.debug_utils_loader.is_some() && self.debug_utils_messenger.is_some() {
                self.debug_utils_loader
                    .as_ref()
                    .unwrap()
                    .destroy_debug_utils_messenger(self.debug_utils_messenger.unwrap(), None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
