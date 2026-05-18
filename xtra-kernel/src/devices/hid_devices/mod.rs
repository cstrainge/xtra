
// Module for handling HID, (Human Interface Devices) such as keyboards and mice. Any number of HID
// devices can be attached to the system at any time. The interfaces here allow for polling and
// async event driven handling of input from those devices. Either from a device specific
// perspective, or from a global perspective where the caller doesn't care about which specific
// device the input is coming from. The latter is useful for things like the console subsystem where
// we just want to know when a key is pressed and don't care which keyboard it came from.

use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// The special modifier keys and their states that are relevant for keyboard input handling.
pub struct KeyModifiers
{
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool
}



/// Callback fired when an attached keyboard device detects a key press.
pub type KeyboardPressedHandler = fn(keyboard_id: usize,
                                     scan_code: &[u8],
                                     modifiers: &KeyModifiers);

/// Callback fired when an attached keyboard device detects a key release.
pub type KeyboardReleasedHandler = fn(keyboard_id: usize,
                                      scan_code: &[u8],
                                      modifiers: &KeyModifiers);



/// The generic interface for a keyboard device driver. This can be implemented by any keyboard
/// device that happens to be attached to the system, regardless of the underlying hardware or
/// communication protocol.
pub trait KeyboardDevice
{
    /// Unique identifier for the keyboard device, this can be used by the caller to keep track of
    /// multiple attached keyboard devices and their associated event handlers.
    fn keyboard_id(&self) -> usize;

    /// Polling interface, Read the current state of the keys pressed down on the keyboard along
    /// with the state of the modifier keys. It is up to the caller to keep track of key releases.
    fn read_keys(&self) -> Option<(&[u8], KeyModifiers)>;

    /// Event driven interface. Register callback handlers for key press and release events.
    ///
    /// This method returns a handler ID that can be used to unregister the event handlers later.
    fn register_key_handlers(&mut self,
                             pressed_handler: KeyboardPressedHandler,
                             released_handler: KeyboardReleasedHandler) -> usize;

    /// Unregister the event handlers for key press and release events using the handler ID returned
    /// from the `register_key_handlers` function.
    fn unregister_key_handlers(&mut self, handler_id: usize);
}



/// Maximum number of buttons that we will support on a mouse device, this is used to size the
/// button array in the `MouseState` struct.
const MAX_MOUSE_BUTTONS: usize = 16;



/// Callback fired when an attached mouse device detects movement.
pub type MouseMovedHandler = fn(mouse_id: usize, delta_x: isize, delta_y: isize);

/// Callback fired when an attached mouse device detects a scroll wheel event.
pub type MouseScrolledHandler = fn(mouse_id: usize, delta: isize);

/// Callback fired when an attached mouse device detects a button press.
pub type MouseButtonPressedHandler = fn(mouse_id: usize, button: &[usize]);

/// Callback fired when an attached mouse device detects a button release.
pub type MouseButtonReleasedHandler = fn(mouse_id: usize, button: &[usize]);



/// The state of a mouse device at a given point in time.
pub struct MouseState
{
    /// The x position of the mouse since the last time it was read.
    pub x: isize,

    /// The y position of the mouse since the last time it was read.
    pub y: isize,

    /// The scroll wheel position of the mouse, this can be positive or negative depending on the
    /// position of the wheel since the last time it was read.
    pub scroll: isize,

    /// The state of the buttons on the mouse, this is a slice of all of the buttons on the mouse.
    /// where 0 is left, 1, the right, and 3 is the middle button. This slice can contain any
    /// number of additional buttons depending on the mouse itself.
    pub buttons: [bool; MAX_MOUSE_BUTTONS]
}



/// The generic interface for a mouse device driver. This can be implemented by any mouse or
/// pointing device that happens to be attached to the system, regardless of the underlying hardware
/// or communication protocol.
pub trait MouseDevice
{
    /// Unique identifier for the mouse device.
    fn mouse_id(&self) -> usize;

    /// Polling interface, read the current state of the mouse device.
    fn read_state(&self) -> MouseState;

    /// Event driven interface. Register callback handlers for mouse movement, scroll, and button
    /// events.
    fn register_mouse_handlers(&mut self,
                               moved_handler: MouseMovedHandler,
                               scrolled_handler: MouseScrolledHandler,
                               button_pressed_handler: MouseButtonPressedHandler,
                               button_released_handler: MouseButtonReleasedHandler) -> usize;

    /// Unregister the event handler functions for the mouse.
    fn unregister_mouse_handlers(&mut self, handler_id: usize);
}



/// Specific driver for USB keyboards.
pub mod usb_keyboard;

/// Specific driver for USB mice.
pub mod usb_mouse;



/// Register the device driver probes for HID devices, such as keyboards, mice, touch pads, etc.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the HID devices discovered in the device tree. If any.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}



/// Callback function type for enumerating all of the attached keyboard devices on the system.
pub type KeyboardEnumerator = fn(keyboard: &dyn KeyboardDevice);



/// Enumerate all of the attached keyboard devices on the system and call the provided callback
/// function for each one.
pub fn enumerate_keyboards(enumerator: KeyboardEnumerator) -> Result<(), &'static str>
{
    Ok(())
}



/// Callback function type for enumerating all of the attached mouse devices on the system.
pub type MouseEnumerator = fn(mouse: &dyn MouseDevice);



/// Enumerate all of the attached mouse devices on the system and call the provided callback
/// function for each one.
pub fn enumerate_mice(enumerator: MouseEnumerator) -> Result<(), &'static str>
{
    Ok(())
}



/// A borrowed view of an attached HID device while enumerating the currently attached devices.
pub enum HidDevice<'device>
{
    /// A keyboard that is currently attached to the system.
    Keyboard(&'device dyn KeyboardDevice),

    /// A mouse that is currently attached to the system.
    Mouse(&'device dyn MouseDevice)
}



/// An attachment event for a supported HID device has occurred.
pub enum Attachment<'device>
{
    /// A supported HID device has been attached to the system.
    Attached(HidDevice<'device>),

    /// A supported HID device has been detached from the system.
    Detached(HidDevice<'device>)
}



/// The event handler function for attachment events.
pub type HidAttachmentHandler = fn(attachment: Attachment);



/// Register a new attachment handler with the HID subsystem. This handler will be called whenever a
/// supported HID device is attached or detached.
pub fn register_hid_attachment_handler(handler: HidAttachmentHandler) -> usize
{
    0
}



/// Unregister an attachment handler from the HID subsystem using the handler ID returned from the
/// `register_hid_attachment_handler` function.
pub fn unregister_hid_attachment_handler(handler_id: usize)
{
}



/// Register a new set of event handlers that will be called when any keyboard attached to the
/// system detects an event.
pub fn register_any_keyboard_handlers(pressed_handler: KeyboardPressedHandler,
                                      released_handler: KeyboardReleasedHandler) -> usize
{
    0
}



/// Unregister a set of event handlers for any keyboard using the handler ID returned from the
/// `register_any_keyboard_handlers` function.
pub fn unregister_any_keyboard_handlers(handler_id: usize)
{
}



/// Register a new set of event handlers that will be called when any mouse attached to the system
/// detects an event.
pub fn register_any_mouse_handlers(moved_handler: MouseMovedHandler,
                                   scrolled_handler: MouseScrolledHandler,
                                   button_pressed_handler: MouseButtonPressedHandler,
                                   button_released_handler: MouseButtonReleasedHandler) -> usize
{
    0
}



/// Unregister a set of event handlers for any mouse using the handler ID returned from the
/// `register_any_mouse_handlers` function.
pub fn unregister_any_mouse_handlers(handler_id: usize)
{
}



/// Run the provided callback function with a reference to the keyboard device associated with the
/// given keyboard ID if it is currently attached to the system.
///
/// Returns true if the callback was called and false if the device was not attached.
pub fn with_attached_keyboard<Handler>(keyboard_id: usize, callback: Handler) -> bool
    where Handler: FnOnce(&dyn KeyboardDevice)
{
    false
}



/// Given the ID of a mouse device, run the provided callback function with a reference to the mouse
///  device if it is still attached to the system.
pub fn with_attached_mouse<Handler>(mouse_id: usize, callback: Handler) -> bool
    where Handler: FnOnce(&dyn MouseDevice)
{
    false
}



/// Enumerate all of the attached keyboards and run the provided callback function for each one.
///
/// If there are no keyboards attached to the system then the callback will not be called at all.
pub fn enumerate_attached_keyboards<Handler>(enumerator: Handler)
    where Handler: FnMut(&dyn KeyboardDevice)
{
}



/// Enumerate all of the attached mice and run the provided callback function for each one.
///
/// If there are no mice attached to the system then the callback will not be called at all.
pub fn enumerate_attached_mice<Handler>(enumerator: Handler)
    where Handler: FnMut(&dyn MouseDevice)
{
}



/// Enumerate all of the attached HID devices and run the provided callback function for each one.
/// This is a more general version of the `enumerate_attached_keyboards` and
/// `enumerate_attached_mice` functions that allows the caller to handle all HID devices in a single
/// callback function
pub fn enumerate_attached_hid_devices<Handler>(mut enumerator: Handler)
    where for<'device> Handler: FnMut(HidDevice<'device>)
{
    enumerate_attached_keyboards(|keyboard|
        {
            enumerator(HidDevice::Keyboard(keyboard))
        });

    enumerate_attached_mice(|mouse|
        {
            enumerator(HidDevice::Mouse(mouse))
        });
}
