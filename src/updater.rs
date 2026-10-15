//! Main-thread Sparkle bridge. The framework lives inside the app bundle;
//! source-only cargo runs remain usable without loading external frameworks.
use objc2::{
    class, msg_send,
    rc::{Allocated, Retained},
    runtime::{AnyClass, AnyObject, Bool},
};
use objc2_foundation::NSString;
use std::{ffi::CString, ptr};

pub struct Updater {
    pub controller: Retained<AnyObject>,
    updater: Retained<AnyObject>,
    _framework: Retained<AnyObject>,
}

impl Updater {
    /// Call only on AppKit's main thread, after creating NSApplication.
    pub unsafe fn new(start: bool) -> Result<Self, String> {
        let bundle: *mut AnyObject = msg_send![class!(NSBundle), mainBundle];
        let path: Option<Retained<NSString>> = msg_send![bundle, privateFrameworksPath];
        let path = path.ok_or("Updates are available in the packaged Dunkerfy.app.")?;
        let framework_path = NSString::from_str(&format!("{}/Sparkle.framework", path));
        let framework: Option<Retained<AnyObject>> =
            msg_send![class!(NSBundle), bundleWithPath: &*framework_path];
        let framework = framework.ok_or("Sparkle.framework is missing. Reinstall Dunkerfy.")?;
        let mut error: *mut AnyObject = ptr::null_mut();
        let loaded: Bool = msg_send![&framework, loadAndReturnError: &mut error];
        if !loaded.as_bool() {
            return Err(error_text(error));
        }
        let name = CString::new("SPUStandardUpdaterController").unwrap();
        let class = AnyClass::get(&name).ok_or("Sparkle's updater is unavailable.")?;
        let alloc: Allocated<AnyObject> = msg_send![class, alloc];
        let controller: Retained<AnyObject> = msg_send![alloc,
            initWithStartingUpdater: Bool::NO,
            updaterDelegate: ptr::null::<AnyObject>(),
            userDriverDelegate: ptr::null::<AnyObject>()];
        let updater: Retained<AnyObject> = msg_send![&controller, updater];
        if start {
            let started: Bool = msg_send![&updater, startUpdater: &mut error];
            if !started.as_bool() {
                return Err(error_text(error));
            }
        }
        Ok(Self {
            controller,
            updater,
            _framework: framework,
        })
    }

    pub unsafe fn checks(&self) -> bool {
        let value: Bool = msg_send![&self.updater, automaticallyChecksForUpdates];
        value.as_bool()
    }
    pub unsafe fn downloads(&self) -> bool {
        let value: Bool = msg_send![&self.updater, automaticallyDownloadsUpdates];
        value.as_bool()
    }
    pub unsafe fn set_checks(&self, enabled: bool) {
        if !enabled {
            self.set_downloads(false);
        }
        let _: () = msg_send![&self.updater, setAutomaticallyChecksForUpdates: Bool::new(enabled)];
    }
    pub unsafe fn set_downloads(&self, enabled: bool) {
        let _: () = msg_send![&self.updater, setAutomaticallyDownloadsUpdates: Bool::new(enabled)];
    }
    pub unsafe fn can_check(&self) -> bool {
        let value: Bool = msg_send![&self.updater, canCheckForUpdates];
        value.as_bool()
    }
    pub unsafe fn check(&self) {
        if self.can_check() {
            let _: () = msg_send![&self.controller, checkForUpdates: ptr::null::<AnyObject>()];
        }
    }
}

unsafe fn error_text(error: *mut AnyObject) -> String {
    if error.is_null() {
        return "The updater could not start. Reinstall Dunkerfy.".into();
    }
    let description: Retained<NSString> = msg_send![error, localizedDescription];
    description.to_string()
}
