use ash::{vk, Entry};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::os;
use std::rc::Rc;
use winit::window::Window;

#[cfg(all(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

const VALIDATION_LAYERS: &[&CStr] =
    &[unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") }];

pub struct VkContext {
    pub window: Rc<Window>,

    pub entry: Entry,
    pub instance: ash::Instance,
    pub debug_utils_fn: Option<ash::ext::debug_utils::Instance>,
    pub debug_utils_messenger: Option<vk::DebugUtilsMessengerEXT>,
    pub surface_fn: Option<ash::khr::surface::Instance>,
    pub surface: Option<vk::SurfaceKHR>,
}

impl VkContext {
    pub fn new(window: &Rc<Window>) -> Self {
        let entry = Entry::linked();
        let instance = Self::create_instance(&entry, window);
        let (debug_utils_fn, debug_utils_messenger) = Self::setup_debug_utils(&entry, &instance);
        let (surface_fn, surface) = Self::create_surface(&entry, &instance, window);

        Self {
            window: window.clone(),
            entry,
            instance,
            debug_utils_fn,
            debug_utils_messenger,
            surface_fn: Some(surface_fn),
            surface: Some(surface),
        }
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
}

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
