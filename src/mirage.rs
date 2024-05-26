use ash::{vk, Entry};
use raw_window_handle;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::borrow::Cow;
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::os;
use std::rc::Rc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use super::gpu;

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
    window: Rc<Window>,
    #[allow(dead_code)]
    entry: Entry,
    // instance: Arc<ash::Instance>,
    instance: Rc<ash::Instance>,
    debug_utils_fn: Option<ash::ext::debug_utils::Instance>,
    debug_utils_messenger: Option<vk::DebugUtilsMessengerEXT>,
    surface_fn: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: ManuallyDrop<Rc<gpu::Device>>,
    swap_chain: ManuallyDrop<Rc<gpu::SwapChain>>,
    forward_renderer: ManuallyDrop<Rc<gpu::ForwardRenderer>>,
    simple_pass: ManuallyDrop<gpu::SimplePass>,
}

impl Mirage {
    pub fn initialize(window: &Rc<Window>) -> Self {
        let entry = Entry::linked();
        // let (event_loop, window) = Self::init_window();
        let instance = Rc::new(Self::create_instance(&entry, &window));
        let (debug_utils_fn, debug_utils_messenger) = Self::setup_debug_utils(&entry, &instance);
        let (surface_fn, surface) = Self::create_surface(&entry, &instance, &window);

        let device = Rc::new(gpu::Device::new(Rc::clone(&instance), &surface_fn, surface));
        let swap_chain = Rc::new(gpu::SwapChain::new(
            &instance,
            &window,
            Rc::clone(&device),
            surface,
        ));
        let forward_renderer = Rc::new(gpu::ForwardRenderer::new(
            &instance,
            Rc::clone(&device),
            Rc::clone(&swap_chain),
        ));
        let mut simple_pass =
            gpu::SimplePass::new(Rc::clone(&device), Rc::clone(&forward_renderer));
        simple_pass.add_object(gpu::SimplePassObject::new(&simple_pass));

        Self {
            entry,
            window: Rc::clone(window),
            debug_utils_fn,
            debug_utils_messenger,
            instance,
            surface_fn,
            surface,
            device: ManuallyDrop::new(device),
            swap_chain: ManuallyDrop::new(swap_chain),
            forward_renderer: ManuallyDrop::new(forward_renderer),
            simple_pass: ManuallyDrop::new(simple_pass),
        }
    }

    pub fn update_window(&self, window: &Window) {}

    pub fn render(&self) {
        self.forward_renderer.render(&self.simple_pass);
    }

    fn create_instance(entry: &Entry, window: &Window) -> ash::Instance {
        if ENABLE_VALIDATION_LAYERS && !Self::check_validation_layers_support(&entry) {
            panic!("Validation layers requested, but not available!")
        }

        unsafe {
            let app_name = CStr::from_bytes_with_nul_unchecked(b"Mirage\0");

            let app_info = vk::ApplicationInfo::default()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 0, 0));

            let layer_names = VALIDATION_LAYERS
                .iter()
                .cloned()
                .map(|layer| layer.as_ptr())
                .collect::<Vec<_>>();

            let mut extension_names =
                ash_window::enumerate_required_extensions(window.display_handle().unwrap().into())
                    .unwrap()
                    .to_vec();

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                extension_names.push(vk::KHR_PORTABILITY_ENUMERATION_NAME.as_ptr());
                // required by *device* extension VK_KHR_portability_subset
                extension_names.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.as_ptr());
            }

            if ENABLE_VALIDATION_LAYERS {
                extension_names.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());
            }

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::default()
            };

            let mut debug_info = Self::build_debug_utils_messenger_create_info();
            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(&layer_names)
                .enabled_extension_names(&extension_names)
                .flags(create_flags)
                .push_next(&mut debug_info);

            entry
                .create_instance(&create_info, None)
                .expect("Instance creation failed")
        }
    }

    fn check_validation_layers_support(entry: &Entry) -> bool {
        unsafe {
            let supported_layers = entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .map(|layer| CStr::from_ptr(layer.layer_name.as_ptr()))
                .collect::<Vec<_>>();

            VALIDATION_LAYERS
                .iter()
                .all(|layer| supported_layers.contains(layer))
        }
    }

    fn setup_debug_utils(
        entry: &Entry,
        instance: &ash::Instance,
    ) -> (
        Option<ash::ext::debug_utils::Instance>,
        Option<vk::DebugUtilsMessengerEXT>,
    ) {
        if !ENABLE_VALIDATION_LAYERS {
            return (None, None);
        }

        unsafe {
            let debug_utils_fn = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let debug_info = Self::build_debug_utils_messenger_create_info();
            let debug_utils_messenger = debug_utils_fn
                .create_debug_utils_messenger(&debug_info, None)
                .expect("failed to setup debug messenger!");

            (Some(debug_utils_fn), Some(debug_utils_messenger))
        }
    }

    fn build_debug_utils_messenger_create_info<'a>() -> vk::DebugUtilsMessengerCreateInfoEXT<'a> {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
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
    }

    fn create_surface(
        entry: &Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> (ash::khr::surface::Instance, vk::SurfaceKHR) {
        unsafe {
            let surface_fn = ash::khr::surface::Instance::new(entry, instance);
            let surface = ash_window::create_surface(
                entry,
                instance,
                window.display_handle().unwrap().into(),
                window.window_handle().unwrap().into(),
                None,
            )
            .unwrap();

            (surface_fn, surface)
        }
    }
}

impl Drop for Mirage {
    fn drop(&mut self) {
        unsafe {
            self.device.device.device_wait_idle().unwrap();

            ManuallyDrop::drop(&mut self.simple_pass);
            ManuallyDrop::drop(&mut self.forward_renderer);
            ManuallyDrop::drop(&mut self.swap_chain);
            ManuallyDrop::drop(&mut self.device);

            self.surface_fn.destroy_surface(self.surface, None);
            if self.debug_utils_fn.is_some() && self.debug_utils_messenger.is_some() {
                self.debug_utils_fn
                    .as_ref()
                    .unwrap()
                    .destroy_debug_utils_messenger(self.debug_utils_messenger.unwrap(), None);
            }
            self.instance.destroy_instance(None);
        }
    }
}
