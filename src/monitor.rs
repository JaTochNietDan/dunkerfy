//! All AX/CF objects live on this worker thread. Only owned Rust strings cross to AppKit.
use dunkerfy::model::{parse_banner, Notification};
use objc2::{class, msg_send, rc::autoreleasepool, runtime::AnyObject};
use objc2_foundation::NSString;
use std::{
    ffi::{c_char, c_void},
    ptr,
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, Instant},
};

type Ref = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> u8;
    static kAXTrustedCheckOptionPrompt: Ref;
    fn AXUIElementCreateApplication(pid: i32) -> Ref;
    fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    fn AXUIElementSetMessagingTimeout(element: Ref, timeout: f32) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(object: Ref);
    static kCFBooleanTrue: Ref;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: Ref,
        value_callbacks: Ref,
    ) -> Ref;
    fn CFRetain(object: Ref) -> Ref;
    fn CFEqual(a: Ref, b: Ref) -> bool;
    fn CFGetTypeID(value: Ref) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFStringCreateWithBytes(
        allocator: Ref,
        bytes: *const u8,
        len: isize,
        encoding: u32,
        external: bool,
    ) -> Ref;
    fn CFStringGetLength(value: Ref) -> isize;
    fn CFStringGetMaximumSizeForEncoding(len: isize, encoding: u32) -> isize;
    fn CFStringGetCString(value: Ref, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFArrayGetCount(array: Ref) -> isize;
    fn CFArrayGetValueAtIndex(array: Ref, index: isize) -> Ref;
}
/// Read permission without displaying a prompt (safe for the polling worker).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Ask macOS once per launch or explicit user action; never from the scan loop.
/// The prompt is asynchronous. Only subsequent trust checks confirm a grant.
pub fn request_access() {
    if is_trusted() {
        return;
    }
    unsafe {
        // Both dictionary entries are process-lifetime CF constants, so no
        // retain/release callbacks are needed for this short-lived dictionary.
        let options = CFDictionaryCreate(
            ptr::null(),
            &kAXTrustedCheckOptionPrompt,
            &kCFBooleanTrue,
            1,
            ptr::null(),
            ptr::null(),
        );
        if !options.is_null() {
            AXIsProcessTrustedWithOptions(options);
            CFRelease(options);
        }
    }
}

const UTF8: u32 = 0x08000100;
struct Owned(Ref);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}
impl Owned {
    fn attribute(&self, name: &str) -> Option<Self> {
        unsafe {
            let key = Owned(CFStringCreateWithBytes(
                ptr::null(),
                name.as_ptr(),
                name.len() as isize,
                UTF8,
                false,
            ));
            let mut result = ptr::null();
            (AXUIElementCopyAttributeValue(self.0, key.0, &mut result) == 0 && !result.is_null())
                .then(|| Owned(result))
        }
    }
    fn text(&self, name: &str) -> String {
        self.attribute(name)
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }
    fn as_string(&self) -> Option<String> {
        unsafe {
            if CFGetTypeID(self.0) != CFStringGetTypeID() {
                return None;
            }
            let size = CFStringGetMaximumSizeForEncoding(CFStringGetLength(self.0), UTF8) + 1;
            if !(1..=65536).contains(&size) {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            if !CFStringGetCString(self.0, data.as_mut_ptr().cast(), size, UTF8) {
                return None;
            }
            let end = data.iter().position(|&c| c == 0).unwrap_or(data.len());
            Some(String::from_utf8_lossy(&data[..end]).into_owned())
        }
    }
    fn array(&self, name: &str) -> Option<Vec<Self>> {
        let array = self.attribute(name)?;
        unsafe {
            if CFGetTypeID(array.0) != CFArrayGetTypeID() {
                return None;
            }
            Some(
                (0..CFArrayGetCount(array.0).min(256))
                    .map(|i| Owned(CFRetain(CFArrayGetValueAtIndex(array.0, i))))
                    .collect(),
            )
        }
    }
    fn same(&self, other: &Self) -> bool {
        unsafe { CFEqual(self.0, other.0) }
    }
}

#[derive(Debug)]
pub enum Message {
    Status(String, bool),
    Event(Notification),
}
struct Seen {
    element: Owned,
    first_seen: Instant,
    delivered: bool,
}
struct Monitor {
    sender: SyncSender<Message>,
    previous: Vec<Seen>,
    pid: Option<i32>,
    primed: bool,
    status: String,
}
impl Monitor {
    fn status(&mut self, text: &str, trusted: bool) {
        if self.status != text
            && self
                .sender
                .try_send(Message::Status(text.into(), trusted))
                .is_ok()
        {
            self.status = text.into();
        }
    }
    fn scan(&mut self) {
        if !is_trusted() {
            self.previous.clear();
            self.primed = false;
            self.status("Accessibility access needed", false);
            return;
        }
        let pid = unsafe {
            let apps: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationsWithBundleIdentifier: &*NSString::from_str("com.apple.notificationcenterui")];
            let count: usize = msg_send![apps, count];
            if count == 0 {
                None
            } else {
                let app: *mut AnyObject = msg_send![apps, objectAtIndex: 0usize];
                Some(msg_send![app, processIdentifier])
            }
        };
        let Some(pid) = pid else {
            self.pid = None;
            self.primed = false;
            self.previous.clear();
            self.status("Waiting for Notification Center", true);
            return;
        };
        if self.pid != Some(pid) {
            self.pid = Some(pid);
            self.primed = false;
            self.previous.clear();
        }
        let root = Owned(unsafe { AXUIElementCreateApplication(pid) });
        unsafe {
            AXUIElementSetMessagingTimeout(root.0, 0.08);
        }
        let Some(windows) = root.array("AXWindows") else {
            self.status("Notification Center is not responding", true);
            return;
        };
        let mut banners = Vec::new();
        let mut budget = Budget {
            remaining: 256,
            deadline: Instant::now() + Duration::from_millis(250),
        };
        for window in windows {
            collect(window, 0, &mut budget, &mut banners);
        }
        // An incomplete traversal must not discard identities and replay old banners next pass.
        if budget.exhausted() {
            self.status("Notification Center scan timed out", true);
            return;
        }
        let mut current = Vec::new();
        for element in banners {
            let previous = self.previous.iter().position(|s| s.element.same(&element));
            let mut seen = if let Some(index) = previous {
                self.previous.swap_remove(index)
            } else {
                Seen {
                    element,
                    first_seen: Instant::now(),
                    delivered: !self.primed,
                }
            };
            // Give the accessibility tree a moment to populate before parsing its content.
            if !seen.delivered && seen.first_seen.elapsed() >= Duration::from_millis(200) {
                let description = seen.element.text("AXDescription");
                let mut values = Vec::new();
                collect_text(&seen.element, 0, &mut budget, &mut values);
                if let Some(event) = parse_banner(&description, &values) {
                    if self.sender.try_send(Message::Event(event)).is_ok() {
                        seen.delivered = true;
                    }
                }
            }
            current.push(seen);
        }
        self.previous = current;
        self.primed = true;
        self.status("Listening for notification banners", true);
    }
}
struct Budget {
    remaining: usize,
    deadline: Instant,
}
impl Budget {
    fn exhausted(&self) -> bool {
        self.remaining == 0 || Instant::now() > self.deadline
    }
    fn take(&mut self) -> bool {
        if self.exhausted() {
            false
        } else {
            self.remaining -= 1;
            true
        }
    }
}
fn collect(element: Owned, depth: usize, budget: &mut Budget, banners: &mut Vec<Owned>) {
    if depth > 16 || !budget.take() {
        return;
    }
    if matches!(
        element.text("AXSubrole").as_str(),
        "AXNotificationCenterBanner" | "AXNotificationCenterAlert"
    ) {
        banners.push(element);
        return;
    }
    for child in element.array("AXChildren").unwrap_or_default() {
        collect(child, depth + 1, budget, banners);
    }
}
fn collect_text(element: &Owned, depth: usize, budget: &mut Budget, values: &mut Vec<String>) {
    if depth > 12 || !budget.take() {
        return;
    }
    if element.text("AXRole") == "AXStaticText" {
        let value = element.text("AXValue");
        if !value.is_empty() {
            values.push(value);
        }
    }
    for child in element.array("AXChildren").unwrap_or_default() {
        collect_text(&child, depth + 1, budget, values);
    }
}
pub fn start() -> Receiver<Message> {
    let (sender, receiver) = mpsc::sync_channel(128);
    std::thread::Builder::new()
        .name("notification-monitor".into())
        .spawn(move || {
            let mut monitor = Monitor {
                sender,
                previous: Vec::new(),
                pid: None,
                primed: false,
                status: String::new(),
            };
            loop {
                autoreleasepool(|_| monitor.scan());
                std::thread::sleep(Duration::from_millis(400));
            }
        })
        .expect("Unable to start notification monitor");
    receiver
}
