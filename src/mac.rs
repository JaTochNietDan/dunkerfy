//! Native AppKit UI. Objective-C messaging is isolated here; application logic is safe Rust.
use crate::monitor::{self, Message};
use dunkerfy::{
    model::{Decision, Engine, Notification, Rule, Settings, Sound},
    storage::Storage,
};
use objc2::{
    class, msg_send,
    rc::{autoreleasepool, Allocated, Retained},
    runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel},
    sel,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::{cell::RefCell, ffi::CString, path::PathBuf, ptr, sync::mpsc::Receiver, time::Instant};
use uuid::Uuid;

#[link(name = "AppKit", kind = "framework")]
extern "C" {}
#[link(name = "ServiceManagement", kind = "framework")]
extern "C" {}

type Obj = Retained<AnyObject>;
thread_local! { static STATE: RefCell<Option<App>> = const { RefCell::new(None) }; }

fn string(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
unsafe fn new(class: &AnyClass) -> Obj {
    msg_send![class, new]
}
unsafe fn frame(class: &AnyClass, r: NSRect) -> Obj {
    let alloc: Allocated<AnyObject> = msg_send![class, alloc];
    msg_send![alloc, initWithFrame: r]
}
unsafe fn set_text(object: &AnyObject, text: &str) {
    let _: () = msg_send![object, setStringValue: &*string(text)];
}
unsafe fn get_text(object: &AnyObject) -> String {
    let value: Retained<NSString> = msg_send![object, stringValue];
    value.to_string()
}
unsafe fn add(parent: &AnyObject, child: &AnyObject) {
    let _: () = msg_send![parent, addSubview: child];
}
unsafe fn label(parent: &AnyObject, text: &str, r: NSRect, size: f64, bold: bool) -> Obj {
    let field: Obj = msg_send![class!(NSTextField), wrappingLabelWithString: &*string(text)];
    let _: () = msg_send![&field, setFrame: r];
    let font: *mut AnyObject = if bold {
        msg_send![class!(NSFont), boldSystemFontOfSize: size]
    } else {
        msg_send![class!(NSFont), systemFontOfSize: size]
    };
    let _: () = msg_send![&field, setFont: font];
    add(parent, &field);
    field
}
unsafe fn secondary(field: &AnyObject) {
    let color: *mut AnyObject = msg_send![class!(NSColor), secondaryLabelColor];
    let _: () = msg_send![field, setTextColor: color];
}
unsafe fn accent() -> *mut AnyObject {
    msg_send![class!(NSColor), colorWithSRGBRed: 0.08f64, green: 0.52f64, blue: 0.46f64, alpha: 1.0f64]
}
unsafe fn panel(parent: &AnyObject, r: NSRect) -> Obj {
    let box_view = frame(class!(NSBox), r);
    let _: () = msg_send![&box_view, setBoxType: 4usize];
    let _: () = msg_send![&box_view, setTitlePosition: 0usize];
    let _: () = msg_send![&box_view, setCornerRadius: 12f64];
    let _: () = msg_send![&box_view, setBorderWidth: 0.6f64];
    let fill: *mut AnyObject = msg_send![class!(NSColor), controlBackgroundColor];
    let edge: *mut AnyObject = msg_send![class!(NSColor), separatorColor];
    let _: () = msg_send![&box_view, setFillColor: fill];
    let _: () = msg_send![&box_view, setBorderColor: edge];
    add(parent, &box_view);
    box_view
}
unsafe fn symbol(button: &AnyObject, name: &str) {
    let image: *mut AnyObject = msg_send![class!(NSImage), imageWithSystemSymbolName: &*string(name), accessibilityDescription: ptr::null::<NSString>()];
    if !image.is_null() {
        let _: () = msg_send![button, setImage: image];
        let _: () = msg_send![button, setImagePosition: 2usize];
    }
}
unsafe fn primary(button: &AnyObject) {
    let _: () = msg_send![button, setBordered: Bool::NO];
    let _: () = msg_send![button, setWantsLayer: Bool::YES];
    let layer: *mut AnyObject = msg_send![button, layer];
    let color: *const std::ffi::c_void = msg_send![accent(), CGColor];
    let _: () = msg_send![layer, setBackgroundColor: color];
    let _: () = msg_send![layer, setCornerRadius: 7f64];
    let white: *mut AnyObject = msg_send![class!(NSColor), whiteColor];
    let _: () = msg_send![button, setContentTintColor: white];
    let title: *mut NSString = msg_send![button, title];
    let attributes: Obj =
        msg_send![class!(NSDictionary), dictionaryWithObject: white, forKey: &*string("NSColor")];
    let allocated: Allocated<AnyObject> = msg_send![class!(NSAttributedString), alloc];
    let attributed: Obj = msg_send![allocated, initWithString: title, attributes: &*attributes];
    let _: () = msg_send![button, setAttributedTitle: &*attributed];
}
unsafe fn button(
    parent: &AnyObject,
    target: &AnyObject,
    title: &str,
    tag: isize,
    r: NSRect,
) -> Obj {
    let b: Obj = msg_send![class!(NSButton), buttonWithTitle: &*string(title), target: target, action: sel!(action:)];
    let _: () = msg_send![&b, setFrame: r];
    let _: () = msg_send![&b, setTag: tag];
    let _: () = msg_send![&b, setBezelStyle: 1usize];
    let font: *mut AnyObject = msg_send![class!(NSFont), systemFontOfSize: 12f64, weight: 0.23f64];
    let _: () = msg_send![&b, setFont: font];
    if tag == 30 || tag == 31 {
        primary(&b);
    }
    add(parent, &b);
    b
}
unsafe fn check(
    parent: &AnyObject,
    target: &AnyObject,
    title: &str,
    tag: isize,
    checked: bool,
    r: NSRect,
) -> Obj {
    let b = button(parent, target, title, tag, r);
    let _: () = msg_send![&b, setButtonType: 3usize];
    let _: () = msg_send![&b, setBordered: Bool::NO];
    let _: () = msg_send![&b, setState: if checked {1isize} else {0isize}];
    b
}
unsafe fn entry(parent: &AnyObject, text: &str, placeholder: &str, r: NSRect) -> Obj {
    let field = frame(class!(NSTextField), r);
    set_text(&field, text);
    let _: () = msg_send![&field, setPlaceholderString: &*string(placeholder)];
    add(parent, &field);
    field
}
unsafe fn slider(parent: &AnyObject, target: &AnyObject, value: f32, tag: isize, r: NSRect) -> Obj {
    let control: Obj = msg_send![class!(NSSlider), sliderWithValue: value as f64, minValue: 0.0f64, maxValue: 1.0f64, target: target, action: sel!(action:)];
    let _: () = msg_send![&control, setFrame: r];
    let _: () = msg_send![&control, setTag: tag];
    add(parent, &control);
    control
}
unsafe fn alert(title: &str, message: &str) {
    let alert = new(class!(NSAlert));
    let _: () = msg_send![&alert, setMessageText: &*string(title)];
    let _: () = msg_send![&alert, setInformativeText: &*string(message)];
    let _: isize = msg_send![&alert, runModal];
}
unsafe fn confirm(title: &str, message: &str) -> bool {
    let alert = new(class!(NSAlert));
    let _: () = msg_send![&alert, setMessageText: &*string(title)];
    let _: () = msg_send![&alert, setInformativeText: &*string(message)];
    let _: *mut AnyObject = msg_send![&alert, addButtonWithTitle: &*string("Delete")];
    let _: *mut AnyObject = msg_send![&alert, addButtonWithTitle: &*string("Cancel")];
    let answer: isize = msg_send![&alert, runModal];
    answer == 1000
}
unsafe fn open_url(url: &str) {
    let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
    let url: *mut AnyObject = msg_send![class!(NSURL), URLWithString: &*string(url)];
    let _: Bool = msg_send![workspace, openURL: url];
}

fn sound_category(sound: &Sound) -> &str {
    match sound {
        Sound::Bundled(id) => dunkerfy::catalog::preset(id)
            .map(|p| p.category.as_str())
            .unwrap_or("Bundled"),
        Sound::System(_) => "macOS",
        Sound::Imported(_) => "Imported",
    }
}
fn sound_detail(sound: &Sound) -> String {
    match sound {
        Sound::Bundled(id) => dunkerfy::catalog::preset(id)
            .map(|p| p.description.clone())
            .unwrap_or_default(),
        Sound::System(_) => "A familiar macOS classic.".into(),
        Sound::Imported(_) => "Your audio, copied into the library.".into(),
    }
}
struct Editor {
    name: Obj,
    app: Obj,
    contains: Obj,
    sound: Obj,
    volume: Obj,
    cooldown: Obj,
    enabled: Obj,
    id: Uuid,
    bundle_id: Option<String>,
    chosen_app_name: String,
    original: Rule,
}
struct Activity {
    app: String,
    detail: String,
    outcome: String,
    time: String,
}
struct App {
    updater: Result<crate::updater::Updater, String>,
    navigation: Vec<Obj>,
    sound_filter: usize,
    sound_query: String,
    sound_grid: Option<Obj>,
    sound_count: Option<Obj>,
    now_playing: Option<Obj>,
    application: Obj,
    target: Obj,
    window: Obj,
    content: Obj,
    status_label: Obj,
    pause_button: Obj,
    _status_item: Obj,
    menu_pause: Obj,
    _timer: Obj,
    settings: Settings,
    storage: Storage,
    engine: Engine,
    receiver: Receiver<Message>,
    page: usize,
    selected: Option<Uuid>,
    editor: Option<Editor>,
    sounds: Vec<(String, Sound)>,
    player: Option<Obj>,
    activity: Vec<Activity>,
    status: String,
    trusted: bool,
    activity_view: Option<Obj>,
}

extern "C" fn action(_: *mut AnyObject, _: Sel, sender: *mut AnyObject) {
    autoreleasepool(|_| {
        STATE.with(|cell| {
            if let Ok(mut state) = cell.try_borrow_mut() {
                if let Some(app) = state.as_mut() {
                    unsafe {
                        let tag: isize = msg_send![sender, tag];
                        app.action(tag, &*sender);
                    }
                }
            }
        })
    });
}
extern "C" fn tick(_: *mut AnyObject, _: Sel, _: *mut AnyObject) {
    autoreleasepool(|_| {
        STATE.with(|cell| {
            if let Ok(mut state) = cell.try_borrow_mut() {
                if let Some(app) = state.as_mut() {
                    unsafe {
                        app.tick();
                    }
                }
            }
        })
    });
}
extern "C" fn reopen(_: *mut AnyObject, _: Sel, _: *mut AnyObject, _: Bool) -> Bool {
    STATE.with(|cell| {
        if let Ok(state) = cell.try_borrow() {
            if let Some(app) = state.as_ref() {
                unsafe {
                    app.show();
                }
            }
        }
    });
    Bool::YES
}

extern "C" fn should_terminate(_: *mut AnyObject, _: Sel, _: *mut AnyObject) -> usize {
    // Sparkle's relaunch also goes through NSApplication termination.
    STATE.with(|state| {
        let Ok(state) = state.try_borrow() else {
            return 0;
        };
        if state
            .as_ref()
            .is_none_or(|app| unsafe { app.may_discard_edits() })
        {
            1
        } else {
            0
        }
    })
}

pub fn run() {
    autoreleasepool(|_| unsafe {
        if !std::env::args().any(|arg| {
            arg == "--smoke-test"
                || arg == "--updater-smoke-test"
                || arg == "--write-icon"
                || arg == "--render-ui"
        }) {
            let bundle: *mut AnyObject = msg_send![class!(NSBundle), mainBundle];
            let identifier: *mut NSString = msg_send![bundle, bundleIdentifier];
            if !identifier.is_null() {
                let apps: *mut AnyObject = msg_send![class!(NSRunningApplication), runningApplicationsWithBundleIdentifier: identifier];
                let count: usize = msg_send![apps, count];
                for index in 0..count {
                    let other: *mut AnyObject = msg_send![apps, objectAtIndex: index];
                    let pid: i32 = msg_send![other, processIdentifier];
                    if pid != std::process::id() as i32 {
                        let _: Bool = msg_send![other, activateWithOptions: 2usize];
                        return;
                    }
                }
            }
        }
        let application: Obj = msg_send![class!(NSApplication), sharedApplication];
        if let Some(path) = std::env::args()
            .skip_while(|arg| arg != "--write-icon")
            .nth(1)
        {
            write_icon(&path);
            return;
        }
        if std::env::args().any(|a| a == "--updater-smoke-test") {
            let updater = crate::updater::Updater::new(true).expect("Packaged updater must start");
            assert!(updater.can_check(), "Updater must allow a manual check");
            println!("Sparkle loaded and started with the packaged feed and public key.");
            return;
        }
        let _: Bool = msg_send![&application, setActivationPolicy: 0isize];
        let name = CString::new("DunkerfyController").unwrap();
        let mut builder =
            ClassBuilder::new(&name, class!(NSObject)).expect("Controller class exists");
        builder.add_method(
            sel!(action:),
            action as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
        );
        builder.add_method(
            sel!(tick:),
            tick as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
        );
        builder.add_method(
            sel!(applicationShouldHandleReopen:hasVisibleWindows:),
            reopen as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, Bool) -> Bool,
        );
        builder.add_method(
            sel!(applicationShouldTerminate:),
            should_terminate as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize,
        );
        let target = new(builder.register());
        let _: () = msg_send![&application, setDelegate: &*target];
        let storage = match Storage::standard() {
            Ok(s) => s,
            Err(e) => {
                alert("Cannot start Dunkerfy", &e);
                return;
            }
        };
        let settings = match if std::env::args().any(|a| a == "--smoke-test" || a == "--render-ui")
        {
            Ok(Settings::default())
        } else {
            storage.load()
        } {
            Ok(s) => s,
            Err(e) => {
                alert("Settings need attention", &e);
                return;
            }
        };
        let alloc: Allocated<AnyObject> = msg_send![class!(NSWindow), alloc];
        let window: Obj = msg_send![alloc, initWithContentRect: rect(0.,0.,980.,660.), styleMask: 7usize, backing: 2usize, defer: Bool::NO];
        let _: () = msg_send![&window, setTitle: &*string("Dunkerfy")];
        let _: () = msg_send![&window, setReleasedWhenClosed: Bool::NO];
        let _: () = msg_send![&window, setTitlebarAppearsTransparent: Bool::YES];
        let _: () = msg_send![&window, center];
        let root: *mut AnyObject = msg_send![&window, contentView];
        let root = &*root;
        let sidebar = frame(class!(NSVisualEffectView), rect(0., 0., 180., 660.));
        let _: () = msg_send![&sidebar, setMaterial: 7isize];
        let _: () = msg_send![&sidebar, setBlendingMode: 0isize];
        add(root, &sidebar);
        let brand = label(&sidebar, "◉", rect(20., 588., 36., 38.), 29., true);
        let _: () = msg_send![&brand, setTextColor: accent()];
        label(&sidebar, "Dunkerfy", rect(57., 597., 110., 27.), 18., true);
        secondary(&label(
            &sidebar,
            "YOUR APPS. YOUR SOUND.",
            rect(22., 565., 150., 20.),
            9.,
            true,
        ));
        let mut navigation = Vec::new();
        for (i, (title, icon)) in [
            ("  Rules", "slider.horizontal.3"),
            ("  Sounds", "waveform"),
            ("  Activity", "bolt.horizontal"),
            ("  Settings", "gearshape"),
            ("  Updates", "arrow.down.circle"),
        ]
        .iter()
        .enumerate()
        {
            let nav = button(
                &sidebar,
                &target,
                title,
                10 + i as isize,
                rect(14., 505. - i as f64 * 43., 152., 34.),
            );
            symbol(&nav, icon);
            let _: () = msg_send![&nav, setAlignment: 0usize];
            navigation.push(nav);
        }
        label(&sidebar, "LISTENER", rect(22., 153., 140., 20.), 9., true);
        let status_label = label(&sidebar, "Starting…", rect(22., 87., 138., 60.), 11., false);
        secondary(&status_label);
        let pause_button = button(
            &sidebar,
            &target,
            if settings.paused {
                "Resume"
            } else {
                "Pause listening"
            },
            20,
            rect(14., 35., 152., 32.),
        );
        let content = frame(class!(NSView), rect(180., 0., 800., 660.));
        add(root, &content);
        let updater = crate::updater::Updater::new(
            !std::env::args().any(|a| a == "--smoke-test" || a == "--render-ui"),
        );
        let menu = new(class!(NSMenu));
        menu_item(&menu, &target, "Open Dunkerfy", 1, "");
        let menu_pause = menu_item(
            &menu,
            &target,
            if settings.paused { "Resume" } else { "Pause" },
            20,
            "",
        );
        let update_item = menu_item(&menu, &target, "Check for Updates…", 80, "");
        if let Ok(ref updater) = updater {
            let _: () = msg_send![&update_item, setTarget: &*updater.controller];
            let _: () = msg_send![&update_item, setAction: sel!(checkForUpdates:)];
        }
        menu_item(&menu, &target, "Quit Dunkerfy", 2, "q");
        let bar: *mut AnyObject = msg_send![class!(NSStatusBar), systemStatusBar];
        let status_item: Obj = msg_send![bar, statusItemWithLength: -1f64];
        let status_button: *mut AnyObject = msg_send![&status_item, button];
        let image: *mut AnyObject = msg_send![class!(NSImage), imageWithSystemSymbolName: &*string("waveform.circle"), accessibilityDescription: &*string("Dunkerfy")];
        let _: () = msg_send![status_button, setImage: image];
        let _: () = msg_send![&status_item, setMenu: &*menu];
        let main_menu = new(class!(NSMenu));
        let app_item = new(class!(NSMenuItem));
        let _: () = msg_send![&app_item, setSubmenu: &*menu];
        let _: () = msg_send![&main_menu, addItem: &*app_item];
        // Standard editing shortcuts for native text fields.
        let edit = new(class!(NSMenu));
        for (title, selector, key) in [
            ("Cut", sel!(cut:), "x"),
            ("Copy", sel!(copy:), "c"),
            ("Paste", sel!(paste:), "v"),
            ("Select All", sel!(selectAll:), "a"),
        ] {
            let item = new(class!(NSMenuItem));
            let _: () = msg_send![&item, setTitle: &*string(title)];
            let _: () = msg_send![&item, setAction: selector];
            let _: () = msg_send![&item, setKeyEquivalent: &*string(key)];
            let _: () = msg_send![&edit, addItem: &*item];
        }
        let edit_item = new(class!(NSMenuItem));
        let _: () = msg_send![&edit_item, setTitle: &*string("Edit")];
        let _: () = msg_send![&edit_item, setSubmenu: &*edit];
        let _: () = msg_send![&main_menu, addItem: &*edit_item];
        let _: () = msg_send![&application, setMainMenu: &*main_menu];
        let timer: Obj = msg_send![class!(NSTimer), scheduledTimerWithTimeInterval: 0.2f64, target: &*target, selector: sel!(tick:), userInfo: ptr::null::<AnyObject>(), repeats: Bool::YES];
        let sounds = storage.sounds();
        let selected = settings.rules.first().map(|r| r.id);
        let mut app = App {
            updater,
            navigation,
            sound_filter: 0,
            sound_query: String::new(),
            sound_grid: None,
            sound_count: None,
            now_playing: None,
            application: application.clone(),
            target,
            window,
            content,
            status_label,
            pause_button,
            _status_item: status_item,
            menu_pause,
            _timer: timer,
            settings,
            storage,
            engine: Engine::default(),
            receiver: if std::env::args().any(|a| a == "--smoke-test" || a == "--render-ui") {
                std::sync::mpsc::channel().1
            } else {
                monitor::start()
            },
            page: 0,
            selected,
            editor: None,
            sounds,
            player: None,
            activity: Vec::new(),
            status: "Starting…".into(),
            trusted: false,
            activity_view: None,
        };
        app.render();
        if let Some(path) = std::env::args().skip_while(|a| a != "--render-ui").nth(1) {
            std::fs::create_dir_all(&path).unwrap();
            app.settings.rules = vec![
                Rule {
                    name: "Team chat".into(),
                    app_name: "Slack".into(),
                    sound: Sound::Bundled("wilhelm".into()),
                    ..Rule::default()
                },
                Rule {
                    name: "Inbox".into(),
                    app_name: "Mail".into(),
                    sound: Sound::Bundled("bubble".into()),
                    ..Rule::default()
                },
                Rule {
                    name: "Heads up".into(),
                    app_name: "Messages".into(),
                    sound: Sound::Bundled("level-up".into()),
                    ..Rule::default()
                },
            ];
            app.selected = Some(app.settings.rules[0].id);
            set_text(&app.status_label, "Preview mode\nSample rules");
            for (mode, appearance_name) in [
                ("light", "NSAppearanceNameAqua"),
                ("dark", "NSAppearanceNameDarkAqua"),
            ] {
                let appearance: *mut AnyObject =
                    msg_send![class!(NSAppearance),appearanceNamed:&*string(appearance_name)];
                let _: () = msg_send![&app.window,setAppearance:appearance];
                let _: () = msg_send![class!(NSAppearance),setCurrentAppearance:appearance];
                for page in 0..5 {
                    app.page = page;
                    app.render();
                    let _: () = msg_send![&app.window, displayIfNeeded];
                    render_view(root, &format!("{path}/{mode}-{page}.png"));
                }
            }
            let _: () = msg_send![&app._timer, invalidate];
            println!("Rendered native UI previews with sample data to {path}");
            return;
        }
        if std::env::args().any(|arg| arg == "--smoke-test") {
            for page in 0..5 {
                app.page = page;
                app.render();
            }
            for preset in dunkerfy::catalog::PRESETS.iter() {
                let path = app
                    .storage
                    .sound_path(&Sound::Bundled(preset.id.clone()))
                    .unwrap();
                let audio =
                    load_audio(&path).expect("Every bundled sound must decode with NSSound");
                let duration: f64 = msg_send![&audio, duration];
                assert!(
                    (duration - preset.duration).abs() < 0.025,
                    "Incorrect duration for {}",
                    preset.id
                );
            }
            app.page = 1;
            app.sound_query = "wilhelm".into();
            app.render();
            assert!(get_text(app.sound_count.as_ref().unwrap()).starts_with("1 SOUNDS"));
            let sender = app.target.clone();
            app.action(4000, &sender);
            assert!(app
                .editor
                .as_ref()
                .is_some_and(|e| e.original.sound == Sound::Bundled("wilhelm".into())));
            app.page = 0;
            app.clear_page();
            app.draw_editor(Rule::default());
            println!(
                "Native AppKit smoke test passed: five pages, editor, menu, login service, updater, 30 decoded sounds, search, and use-sound draft."
            );
            let _: () = msg_send![&app._timer, invalidate];
            return;
        }
        app.show();
        STATE.with(|state| *state.borrow_mut() = Some(app));
        let _: () = msg_send![&application, run];
    });
}
unsafe fn menu_item(
    menu: &AnyObject,
    target: &AnyObject,
    title: &str,
    tag: isize,
    key: &str,
) -> Obj {
    let item = new(class!(NSMenuItem));
    let _: () = msg_send![&item, setTitle: &*string(title)];
    let _: () = msg_send![&item, setTag: tag];
    let _: () = msg_send![&item, setTarget: target];
    let _: () = msg_send![&item, setAction: sel!(action:)];
    let _: () = msg_send![&item, setKeyEquivalent: &*string(key)];
    let _: () = msg_send![menu, addItem: &*item];
    item
}

impl App {
    unsafe fn show(&self) {
        let _: () = msg_send![&self.window, makeKeyAndOrderFront: ptr::null::<AnyObject>()];
        let _: () = msg_send![&self.application, activateIgnoringOtherApps: Bool::YES];
    }
    unsafe fn commit(&mut self, settings: Settings) -> bool {
        match self.storage.save(&settings) {
            Ok(()) => {
                self.settings = settings;
                true
            }
            Err(e) => {
                alert("Could not save", &e);
                false
            }
        }
    }
    unsafe fn clear_page(&mut self) {
        self.sound_grid = None;
        self.sound_count = None;
        self.now_playing = None;
        self.editor = None;
        self.activity_view = None;
        let subviews: Obj = msg_send![&self.content, subviews];
        let copy: Obj = msg_send![&subviews, copy];
        let count: usize = msg_send![&copy, count];
        for i in 0..count {
            let v: *mut AnyObject = msg_send![&copy, objectAtIndex: i];
            let _: () = msg_send![v, removeFromSuperview];
        }
        let background = panel(&self.content, rect(0., 0., 800., 660.));
        let fill: *mut AnyObject = msg_send![class!(NSColor), windowBackgroundColor];
        let _: () = msg_send![&background,setFillColor:fill];
        let _: () = msg_send![&background,setBorderWidth:0f64];
        let _: () = msg_send![&background,setCornerRadius:0f64];
    }
    unsafe fn header(&self, title: &str, subtitle: &str) {
        label(&self.content, title, rect(24., 597., 752., 34.), 24., true);
        secondary(&label(
            &self.content,
            subtitle,
            rect(24., 558., 752., 32.),
            12.,
            false,
        ));
    }
    unsafe fn render(&mut self) {
        self.clear_page();
        for (i, nav) in self.navigation.iter().enumerate() {
            let color: *mut AnyObject = if i == self.page {
                accent()
            } else {
                msg_send![class!(NSColor), labelColor]
            };
            let _: () = msg_send![nav, setContentTintColor: color];
            let _: () = msg_send![nav, setBordered: Bool::new(i == self.page)];
        }
        match self.page {
            0 => self.rules_page(),
            1 => self.sounds_page(),
            2 => self.activity_page(),
            3 => self.settings_page(),
            _ => self.updates_page(),
        }
    }
    unsafe fn rules_page(&mut self) {
        self.header(
            "Sound rules",
            "A familiar sound for every app. More specific rules go first.",
        );
        button(
            &self.content,
            &self.target,
            "+ New rule",
            30,
            rect(22., 510., 128., 32.),
        );
        secondary(&label(
            &self.content,
            &format!("{} rules · first match wins", self.settings.rules.len()),
            rect(164., 514., 150., 22.),
            11.,
            false,
        ));
        panel(&self.content, rect(24., 81., 288., 415.));
        let height = (self.settings.rules.len().max(1) as f64 * 66.).max(395.);
        let list = scroll(&self.content, rect(30., 91., 276., 395.), height);
        if self.settings.rules.is_empty() {
            label(
                &list,
                "A quieter kind of control.",
                rect(15., height - 55., 245., 25.),
                14.,
                true,
            );
            secondary(&label(
                &list,
                "Start with one app. You can add keywords and fine-tune things later.",
                rect(15., height - 125., 240., 58.),
                12.,
                false,
            ));
        }
        for (i, rule) in self.settings.rules.iter().enumerate() {
            let y = height - (i + 1) as f64 * 66.;
            let row = button(
                &list,
                &self.target,
                &format!("{}  {}", if rule.enabled { "●" } else { "○" }, rule.name),
                1000 + i as isize,
                rect(8., y + 29., 250., 28.),
            );
            let _: () = msg_send![&row, setAlignment: 0usize];
            let _: () = msg_send![&row, setBordered: Bool::new(Some(rule.id)==self.selected)];
            if Some(rule.id) == self.selected {
                let _: () = msg_send![&row, setContentTintColor: accent()];
            }
            secondary(&label(
                &list,
                &format!(
                    "{} · {}",
                    rule.app_name,
                    if rule.contains.is_empty() {
                        "All notifications"
                    } else {
                        &rule.contains
                    }
                ),
                rect(18., y + 5., 235., 20.),
                11.,
                false,
            ));
        }
        button(
            &self.content,
            &self.target,
            "↑ Earlier",
            33,
            rect(22., 29., 137., 30.),
        );
        button(
            &self.content,
            &self.target,
            "↓ Later",
            34,
            rect(171., 29., 143., 30.),
        );
        if let Some(rule) = self
            .selected
            .and_then(|id| self.settings.rules.iter().find(|r| r.id == id).cloned())
        {
            self.draw_editor(rule);
        } else {
            panel(&self.content, rect(330., 29., 446., 513.));
            let mark = label(&self.content, "♫", rect(367., 379., 360., 65.), 49., true);
            let _: () = msg_send![&mark,setTextColor:accent()];
            label(
                &self.content,
                "Make it sound like you.",
                rect(367., 320., 365., 34.),
                22.,
                true,
            );
            secondary(&label(
                &self.content,
                "A subtle ping for Mail. A Wilhelm scream for the group chat. Your call.",
                rect(367., 248., 354., 60.),
                14.,
                false,
            ));
            button(
                &self.content,
                &self.target,
                "Create a rule",
                30,
                rect(365., 194., 158., 34.),
            );
            button(
                &self.content,
                &self.target,
                "Explore sounds",
                11,
                rect(533., 194., 171., 34.),
            );
        }
    }
    unsafe fn draw_editor(&mut self, rule: Rule) {
        panel(&self.content, rect(330., 29., 446., 513.));
        label(
            &self.content,
            "RULE DETAILS",
            rect(350., 506., 220., 20.),
            10.,
            true,
        );
        let enabled = check(
            &self.content,
            &self.target,
            "Enabled",
            49,
            rule.enabled,
            rect(669., 502., 92., 25.),
        );
        label(
            &self.content,
            "Name",
            rect(350., 471., 390., 18.),
            11.,
            true,
        );
        let name = entry(
            &self.content,
            &rule.name,
            "Work messages",
            rect(350., 439., 406., 27.),
        );
        label(
            &self.content,
            "Application",
            rect(350., 406., 390., 18.),
            11.,
            true,
        );
        let app = entry(
            &self.content,
            &rule.app_name,
            "Name shown on the notification",
            rect(350., 374., 278., 27.),
        );
        button(
            &self.content,
            &self.target,
            "Choose…",
            40,
            rect(637., 372., 121., 31.),
        );
        label(
            &self.content,
            "Contains a keyword  ·  optional",
            rect(350., 342., 390., 18.),
            11.,
            true,
        );
        let contains = entry(
            &self.content,
            &rule.contains,
            "Person, channel, or keyword",
            rect(350., 310., 406., 27.),
        );
        label(
            &self.content,
            "Sound",
            rect(350., 278., 390., 18.),
            11.,
            true,
        );
        let popup = frame(class!(NSPopUpButton), rect(347., 243., 297., 30.));
        let mut found = false;
        for (i, (name, sound)) in self.sounds.iter().enumerate() {
            let title = format!("{} · {}", name, sound_category(sound));
            let _: () = msg_send![&popup,addItemWithTitle:&*string(&title)];
            if *sound == rule.sound {
                let _: () = msg_send![&popup,selectItemAtIndex:i as isize];
                found = true;
            }
        }
        if !found {
            self.sounds
                .push(("Missing sound — choose another".into(), rule.sound.clone()));
            let _: () =
                msg_send![&popup,addItemWithTitle:&*string("Missing sound — choose another")];
            let _: () = msg_send![&popup,selectItemAtIndex:(self.sounds.len()-1) as isize];
        }
        add(&self.content, &popup);
        button(
            &self.content,
            &self.target,
            "▶ Preview",
            41,
            rect(652., 242., 106., 31.),
        );
        label(
            &self.content,
            "Volume",
            rect(350., 209., 100., 18.),
            11.,
            true,
        );
        let volume = slider(
            &self.content,
            &self.target,
            rule.volume,
            49,
            rect(437., 206., 318., 24.),
        );
        label(
            &self.content,
            "Cooldown",
            rect(350., 163., 110., 20.),
            11.,
            true,
        );
        let cooldown = entry(
            &self.content,
            &rule.cooldown_secs.to_string(),
            "1",
            rect(445., 160., 62., 26.),
        );
        secondary(&label(
            &self.content,
            "seconds between sounds",
            rect(519., 160., 235., 23.),
            11.,
            false,
        ));
        secondary(&label(
            &self.content,
            "Changes apply when you save.",
            rect(350., 113., 390., 20.),
            11.,
            false,
        ));
        button(
            &self.content,
            &self.target,
            "Delete",
            32,
            rect(347., 56., 90., 32.),
        );
        button(
            &self.content,
            &self.target,
            "Test rule",
            42,
            rect(524., 56., 110., 32.),
        );
        let save = button(
            &self.content,
            &self.target,
            "Save rule",
            31,
            rect(644., 56., 114., 32.),
        );
        let _: () = msg_send![&save,setKeyEquivalent:&*string("\r")];
        self.editor = Some(Editor {
            name,
            app,
            contains,
            sound: popup,
            volume,
            cooldown,
            enabled,
            id: rule.id,
            bundle_id: rule.bundle_id.clone(),
            chosen_app_name: rule.app_name.clone(),
            original: rule,
        });
    }
    unsafe fn read_editor(&self) -> Result<Rule, String> {
        let e = self.editor.as_ref().ok_or("Select a rule first.")?;
        let index: isize = msg_send![&e.sound, indexOfSelectedItem];
        let sound = self
            .sounds
            .get(index as usize)
            .ok_or("Choose a sound.")?
            .1
            .clone();
        let volume: f64 = msg_send![&e.volume, doubleValue];
        let state: isize = msg_send![&e.enabled, state];
        let app_name = get_text(&e.app).trim().to_owned();
        // Keep the selected bundle identity only while its display name is unchanged.
        let bundle_id = if e.chosen_app_name == app_name {
            e.bundle_id.clone()
        } else {
            None
        };
        let rule = Rule {
            id: e.id,
            name: get_text(&e.name).trim().into(),
            app_name,
            bundle_id,
            contains: get_text(&e.contains).trim().into(),
            sound,
            volume: volume as f32,
            cooldown_secs: get_text(&e.cooldown)
                .parse()
                .map_err(|_| "Enter a whole number for the cooldown.")?,
            enabled: state == 1,
        };
        rule.validate()?;
        Ok(rule)
    }
    unsafe fn sounds_page(&mut self) {
        self.header(
            "Sound library",
            "30 ready-to-play presets. A little useful. A little ridiculous.",
        );
        let search = frame(class!(NSSearchField), rect(24., 508., 461., 30.));
        set_text(&search, &self.sound_query);
        let _: () =
            msg_send![&search,setPlaceholderString:&*string("Find a sound, mood, or category")];
        let _: () = msg_send![&search,setTarget:&*self.target];
        let _: () = msg_send![&search,setAction:sel!(action:)];
        let _: () = msg_send![&search,setTag:51isize];
        let _: () = msg_send![&search,setSendsSearchStringImmediately:Bool::YES];
        add(&self.content, &search);
        button(
            &self.content,
            &self.target,
            "Import audio…",
            50,
            rect(619., 507., 159., 32.),
        );
        for (i, title) in ["All", "Fun", "Arcade", "Calm", "macOS", "Imported"]
            .iter()
            .enumerate()
        {
            let filter = button(
                &self.content,
                &self.target,
                title,
                90 + i as isize,
                rect(22. + i as f64 * 105., 459., 98., 29.),
            );
            if self.sound_filter == i {
                primary(&filter);
            }
        }
        let count = label(&self.content, "", rect(24., 425., 750., 20.), 10., true);
        secondary(&count);
        self.sound_count = Some(count);
        let grid = frame(class!(NSView), rect(18., 89., 764., 328.));
        add(&self.content, &grid);
        self.sound_grid = Some(grid);
        self.render_sound_grid();
        panel(&self.content, rect(24., 23., 752., 55.));
        let now = label(
            &self.content,
            "Press play to find your next notification sound.",
            rect(40., 39., 590., 24.),
            12.,
            false,
        );
        secondary(&now);
        self.now_playing = Some(now);
        button(
            &self.content,
            &self.target,
            "Stop",
            52,
            rect(679., 34., 81., 30.),
        );
    }
    unsafe fn render_sound_grid(&self) {
        let Some(grid) = &self.sound_grid else {
            return;
        };
        let children: *mut AnyObject = msg_send![grid, subviews];
        let copy: Obj = msg_send![children, copy];
        let count: usize = msg_send![&copy, count];
        for i in 0..count {
            let child: *mut AnyObject = msg_send![&copy,objectAtIndex:i];
            let _: () = msg_send![child, removeFromSuperview];
        }
        let categories = ["All", "Fun", "Arcade", "Calm", "macOS", "Imported"];
        let words: Vec<_> = self
            .sound_query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let matches: Vec<_> = self
            .sounds
            .iter()
            .enumerate()
            .filter(|(_, (name, sound))| {
                let category = sound_category(sound);
                let haystack =
                    format!("{} {} {}", name, category, sound_detail(sound)).to_lowercase();
                (self.sound_filter == 0 || category == categories[self.sound_filter])
                    && words.iter().all(|word| haystack.contains(word))
            })
            .collect();
        if let Some(count) = &self.sound_count {
            set_text(
                count,
                &format!(
                    "{} SOUNDS  /  {}",
                    matches.len(),
                    categories[self.sound_filter].to_uppercase()
                ),
            );
        }
        let rows = matches.len().div_ceil(2);
        let height = (rows as f64 * 110.).max(328.);
        let list = scroll(grid, rect(0., 0., 764., 328.), height);
        if matches.is_empty() {
            label(
                &list,
                "Nothing here yet.",
                rect(22., 245., 690., 30.),
                18.,
                true,
            );
            secondary(&label(
                &list,
                "Try a different search or category, or import your own audio.",
                rect(22., 200., 690., 30.),
                13.,
                false,
            ));
        }
        for (position, (index, (name, sound))) in matches.iter().enumerate() {
            let x = 6. + (position % 2) as f64 * 374.;
            let y = height - (position / 2 + 1) as f64 * 110. + 8.;
            panel(&list, rect(x, y, 358., 100.));
            label(&list, name, rect(x + 15., y + 66., 270., 23.), 14., true);
            let detail = sound_detail(sound);
            secondary(&label(
                &list,
                &detail,
                rect(x + 15., y + 40., 334., 23.),
                11.,
                false,
            ));
            let category = sound_category(sound);
            let category_label = label(
                &list,
                category,
                rect(x + 15., y + 11., 105., 20.),
                10.,
                true,
            );
            let _: () = msg_send![&category_label,setTextColor:accent()];
            let preview = button(
                &list,
                &self.target,
                "Play",
                2000 + *index as isize,
                rect(x + 119., y + 7., 79., 27.),
            );
            symbol(&preview, "play.fill");
            let _: () =
                msg_send![&preview, setAccessibilityLabel: &*string(&format!("Preview {name}"))];
            button(
                &list,
                &self.target,
                "Use",
                4000 + *index as isize,
                rect(x + 204., y + 7., 64., 27.),
            );
            if matches!(sound, Sound::Imported(_)) {
                button(
                    &list,
                    &self.target,
                    "Delete",
                    3000 + *index as isize,
                    rect(x + 274., y + 7., 77., 27.),
                );
            } else if let Sound::Bundled(id) = sound {
                if let Some(preset) = dunkerfy::catalog::preset(id) {
                    secondary(&label(
                        &list,
                        &format!("{:.1}s", preset.duration),
                        rect(x + 291., y + 10., 57., 20.),
                        10.,
                        false,
                    ));
                }
            }
        }
    }
    unsafe fn activity_page(&mut self) {
        self.header(
            "Activity",
            "A live view of what Dunkerfy hears. Only the last 60 events stay in memory.",
        );
        button(
            &self.content,
            &self.target,
            "Clear activity",
            60,
            rect(622., 508., 156., 32.),
        );
        secondary(&label(
            &self.content,
            "LOCAL TO THIS SESSION",
            rect(24., 513., 530., 20.),
            10.,
            true,
        ));
        let container = frame(class!(NSView), rect(18., 25., 764., 467.));
        add(&self.content, &container);
        self.activity_view = Some(container);
        self.refresh_activity();
    }
    unsafe fn refresh_activity(&self) {
        if let Some(view) = &self.activity_view {
            let children: *mut AnyObject = msg_send![view, subviews];
            let copy: Obj = msg_send![children, copy];
            let count: usize = msg_send![&copy, count];
            for i in 0..count {
                let child: *mut AnyObject = msg_send![&copy,objectAtIndex:i];
                let _: () = msg_send![child, removeFromSuperview];
            }
            if self.activity.is_empty() {
                panel(view, rect(6., 10., 752., 445.));
                let icon = label(view, "◌", rect(38., 308., 650., 55.), 42., true);
                let _: () = msg_send![&icon,setTextColor:accent()];
                label(
                    view,
                    "Listening for the next little thing.",
                    rect(38., 258., 650., 32.),
                    21.,
                    true,
                );
                secondary(&label(view,"Enable Accessibility access, then trigger a notification from an app with banners turned on. Events will appear here.",rect(38.,173.,620.,67.),13.,false));
                button(
                    view,
                    &self.target,
                    "Open settings",
                    13,
                    rect(36., 112., 158., 32.),
                );
                return;
            }
            let height = self
                .activity
                .iter()
                .map(|a| if a.detail.is_empty() { 86. } else { 134. })
                .sum::<f64>()
                .max(467.);
            let list = scroll(view, rect(0., 0., 764., 467.), height);
            let mut y = height;
            for event in &self.activity {
                let h = if event.detail.is_empty() { 86. } else { 134. };
                y -= h;
                panel(&list, rect(6., y + 7., 732., h - 12.));
                label(
                    &list,
                    &event.app,
                    rect(22., y + h - 38., 550., 23.),
                    13.,
                    true,
                );
                secondary(&label(
                    &list,
                    &event.time,
                    rect(644., y + h - 37., 80., 20.),
                    11.,
                    false,
                ));
                secondary(&label(
                    &list,
                    &event.outcome,
                    rect(22., y + h - 62., 690., 20.),
                    12.,
                    false,
                ));
                if !event.detail.is_empty() {
                    secondary(&label(
                        &list,
                        &event.detail,
                        rect(22., y + 15., 690., 48.),
                        11.,
                        false,
                    ));
                }
            }
        }
    }
    unsafe fn settings_page(&self) {
        self.header("Settings", "Set it up once. Let your apps do the talking.");
        panel(&self.content, rect(24., 377., 752., 165.));
        label(
            &self.content,
            "01  Accessibility",
            rect(43., 499., 500., 24.),
            15.,
            true,
        );
        secondary(&label(
            &self.content,
            "Allow Dunkerfy to read visible notification banners and match your sound rules.",
            rect(43., 447., 690., 42.),
            12.,
            false,
        ));
        button(
            &self.content,
            &self.target,
            "Open Accessibility settings",
            70,
            rect(40., 394., 246., 32.),
        );
        panel(&self.content, rect(24., 199., 752., 165.));
        label(
            &self.content,
            "02  Original notification sounds",
            rect(43., 321., 690., 24.),
            15.,
            true,
        );
        secondary(&label(&self.content,"For each app, turn off “Play sound for notification” in macOS. Keep banners enabled. Dunkerfy adds your sound; it cannot mute the original.",rect(43.,267.,690.,44.),12.,false));
        button(
            &self.content,
            &self.target,
            "Open Notification settings",
            71,
            rect(40., 216., 246., 32.),
        );
        panel(&self.content, rect(24., 25., 752., 161.));
        check(
            &self.content,
            &self.target,
            "Launch at login",
            73,
            login_enabled(),
            rect(42., 141., 690., 25.),
        );
        check(
            &self.content,
            &self.target,
            "Show notification text in the temporary activity log",
            72,
            self.settings.show_notification_text,
            rect(42., 104., 690., 25.),
        );
        label(
            &self.content,
            "Master volume",
            rect(43., 58., 147., 22.),
            12.,
            true,
        );
        slider(
            &self.content,
            &self.target,
            self.settings.master_volume,
            74,
            rect(209., 56., 527., 25.),
        );
    }
    unsafe fn updates_page(&self) {
        self.header(
            "Updates",
            "Keep things fresh. Your rules and sounds stay right where they are.",
        );
        panel(&self.content, rect(24., 178., 752., 364.));
        label(
            &self.content,
            concat!("Dunkerfy ", env!("CARGO_PKG_VERSION")),
            rect(44., 489., 690., 31.),
            21.,
            true,
        );
        secondary(&label(
            &self.content,
            "DEVELOPER PREVIEW",
            rect(45., 461., 680., 20.),
            10.,
            true,
        ));
        match &self.updater {
            Ok(updater) => {
                button(
                    &self.content,
                    &self.target,
                    "Check for Updates…",
                    80,
                    rect(41., 402., 223., 34.),
                );
                check(
                    &self.content,
                    &self.target,
                    "Automatically check for updates",
                    81,
                    updater.checks(),
                    rect(44., 346., 690., 25.),
                );
                let downloads = check(
                    &self.content,
                    &self.target,
                    "Automatically download and install updates",
                    82,
                    updater.downloads(),
                    rect(44., 309., 690., 25.),
                );
                let _: () = msg_send![&downloads,setEnabled:Bool::new(updater.checks())];
                secondary(&label(&self.content,"Automatic checks are opt-in and run about once a day. Every update is verified before installation. You can always check manually.",rect(45.,216.,682.,67.),12.,false));
            }
            Err(error) => {
                secondary(&label(
                    &self.content,
                    error,
                    rect(44., 302., 690., 114.),
                    13.,
                    false,
                ));
            }
        }
        secondary(&label(&self.content,"Checks contact dunkerfy.oleary.club and GitHub. Notification text, rules, and sound files are never sent. No analytics.",rect(26.,100.,744.,55.),12.,false));
        secondary(&label(&self.content,"Live notification capture still needs verification. See the release notes for signing and validation status.",rect(26.,35.,744.,48.),11.,false));
    }
    unsafe fn may_discard_edits(&self) -> bool {
        let Some(e) = &self.editor else {
            return true;
        };
        let volume: f64 = msg_send![&e.volume, doubleValue];
        let enabled: isize = msg_send![&e.enabled, state];
        let index: isize = msg_send![&e.sound, indexOfSelectedItem];
        let dirty = get_text(&e.name) != e.original.name
            || get_text(&e.app) != e.original.app_name
            || get_text(&e.contains) != e.original.contains
            || get_text(&e.cooldown) != e.original.cooldown_secs.to_string()
            || (volume as f32 - e.original.volume).abs() > 0.001
            || (enabled == 1) != e.original.enabled
            || self.sounds.get(index as usize).map(|s| &s.1) != Some(&e.original.sound);
        if !dirty {
            return true;
        }
        let dialog = new(class!(NSAlert));
        let _: () = msg_send![&dialog, setMessageText: &*string("Discard unsaved changes?")];
        let _: () = msg_send![&dialog, setInformativeText: &*string("Choose Cancel to return to your rule and save it first.")];
        let _: *mut AnyObject = msg_send![&dialog, addButtonWithTitle: &*string("Cancel")];
        let _: *mut AnyObject = msg_send![&dialog, addButtonWithTitle: &*string("Discard changes")];
        let answer: isize = msg_send![&dialog, runModal];
        answer == 1001
    }
    unsafe fn action(&mut self, tag: isize, sender: &AnyObject) {
        if matches!(tag, 10..=14 | 30 | 33..=34 | 1000..=1999) && !self.may_discard_edits() {
            return;
        }
        match tag {
            1 => self.show(),
            2 => {
                let _: () = msg_send![&self.application, performSelector: sel!(terminate:), withObject: ptr::null::<AnyObject>(), afterDelay: 0.0f64];
            }
            10..=14 => {
                self.page = (tag - 10) as usize;
                self.render();
            }
            20 => {
                let mut s = self.settings.clone();
                s.paused = !s.paused;
                if self.commit(s) {
                    let title = if self.settings.paused {
                        "Resume listening"
                    } else {
                        "Pause listening"
                    };
                    let _: () = msg_send![&self.pause_button,setTitle:&*string(title)];
                    let _: () = msg_send![&self.menu_pause,setTitle:&*string(if self.settings.paused{"Resume"}else{"Pause"})];
                    if self.settings.paused {
                        self.stop_audio();
                    }
                    self.update_status();
                }
            }
            30 => {
                self.page = 0;
                self.selected = None;
                self.render();
                // The empty-state card is replaced by the editable draft.
                self.clear_page();
                self.header(
                    "New sound rule",
                    "Choose an app and a sound. Save when it feels right.",
                );
                label(
                    &self.content,
                    "One app. Its own voice.",
                    rect(24., 456., 280., 32.),
                    18.,
                    true,
                );
                secondary(&label(&self.content,"Use the exact name shown on its notifications, or choose the application.\n\nLeave the keyword blank to match every notification.",rect(24.,278.,265.,160.),13.,false));
                self.draw_editor(Rule::default());
            }
            31 => match self.read_editor() {
                Ok(rule) => {
                    if let Err(e) = self.storage.sound_path(&rule.sound).and_then(|p| {
                        if p.is_file() {
                            Ok(p)
                        } else {
                            Err("The sound file is missing. Choose another sound.".into())
                        }
                    }) {
                        alert("Sound unavailable", &e);
                        return;
                    }
                    let mut s = self.settings.clone();
                    if let Some(existing) = s.rules.iter_mut().find(|r| r.id == rule.id) {
                        *existing = rule.clone();
                    } else {
                        s.rules.push(rule.clone());
                    }
                    if self.commit(s) {
                        self.selected = Some(rule.id);
                        self.engine.reset();
                        self.render();
                    }
                }
                Err(e) => alert("Check your rule", &e),
            },
            32 => {
                let Some(id) = self.editor.as_ref().map(|e| e.id) else {
                    return;
                };
                if confirm(
                    "Delete this rule?",
                    "This removes the rule. Your imported sounds stay in the library.",
                ) {
                    let mut s = self.settings.clone();
                    s.rules.retain(|r| r.id != id);
                    if self.commit(s) {
                        self.selected = self.settings.rules.first().map(|r| r.id);
                        self.render();
                    }
                }
            }
            33 | 34 => {
                let Some(index) = self
                    .selected
                    .and_then(|id| self.settings.rules.iter().position(|r| r.id == id))
                else {
                    return;
                };
                let other = if tag == 33 {
                    index.checked_sub(1)
                } else {
                    Some(index + 1).filter(|&i| i < self.settings.rules.len())
                };
                if let Some(other) = other {
                    let mut s = self.settings.clone();
                    s.rules.swap(index, other);
                    if self.commit(s) {
                        self.render();
                    }
                }
            }
            40 => {
                if let Some(path) =
                    choose_file("Choose an application", "Choose app", Some("/Applications"))
                {
                    let bundle: *mut AnyObject = msg_send![class!(NSBundle),bundleWithPath:&*string(&path.to_string_lossy())];
                    if bundle.is_null() {
                        alert("Choose an application", "Select a .app bundle.");
                        return;
                    }
                    let identifier: *mut NSString = msg_send![bundle, bundleIdentifier];
                    if identifier.is_null() {
                        alert(
                            "Choose an application",
                            "This file is not an application bundle.",
                        );
                        return;
                    }
                    let display: *mut NSString = msg_send![bundle,objectForInfoDictionaryKey:&*string("CFBundleDisplayName")];
                    let name = if display.is_null() {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        (&*display).to_string()
                    };
                    if let Some(e) = self.editor.as_mut() {
                        set_text(&e.app, &name);
                        e.bundle_id = Some((&*identifier).to_string());
                        e.chosen_app_name = name;
                    }
                }
            }
            41 => {
                if let Some(e) = &self.editor {
                    let index: isize = msg_send![&e.sound, indexOfSelectedItem];
                    let volume: f64 = msg_send![&e.volume, doubleValue];
                    if let Some((_, sound)) = self.sounds.get(index as usize).cloned() {
                        if let Err(e) = self.play(&sound, volume as f32) {
                            alert("Could not play sound", &e);
                        }
                    }
                }
            }
            42 => match self.read_editor() {
                Ok(rule) => {
                    let mut s = self.settings.clone();
                    s.rules = vec![rule.clone()];
                    s.paused = false;
                    let n = Notification {
                        app_name: rule.app_name.clone(),
                        bundle_id: rule.bundle_id.clone(),
                        text: format!("Test notification {}", rule.contains),
                    };
                    let decision = Engine::default().evaluate(&n, &s, Instant::now());
                    if matches!(decision, Decision::Play(_)) {
                        let result = self.play(&rule.sound, rule.volume);
                        self.record(
                            &n,
                            match result {
                                Ok(()) => "Simulated test · sound played".into(),
                                Err(e) => format!("Simulated test · {e}"),
                            },
                        );
                    } else {
                        alert("Rule is disabled", "Enable this rule to test matching.");
                    }
                }
                Err(e) => alert("Check your rule", &e),
            },
            49 => {}
            50 => {
                if let Some(path) = choose_file("Import a notification sound", "Import", None) {
                    if load_audio(&path).is_none() {
                        alert("Unsupported audio","macOS could not decode this file. Choose AIFF, WAV, MP3, M4A, or CAF audio.");
                        return;
                    }
                    match self.storage.import_sound(&path) {
                        Ok(_) => {
                            self.sounds = self.storage.sounds();
                            self.render();
                        }
                        Err(e) => alert("Could not import audio", &e),
                    }
                }
            }
            51 => {
                self.sound_query = get_text(sender);
                self.render_sound_grid();
            }
            52 => self.stop_audio(),
            90..=95 => {
                self.sound_filter = (tag - 90) as usize;
                self.render();
            }
            4000..=4999 => {
                if let Some((name, sound)) = self.sounds.get((tag - 4000) as usize).cloned() {
                    self.stop_audio();
                    self.page = 0;
                    self.selected = None;
                    self.render();
                    self.clear_page();
                    self.header(
                        "New sound rule",
                        "Your sound is picked. Now give it an app.",
                    );
                    label(&self.content, &name, rect(24., 456., 280., 32.), 20., true);
                    secondary(&label(
                        &self.content,
                        &sound_detail(&sound),
                        rect(24., 385., 265., 60.),
                        13.,
                        false,
                    ));
                    self.draw_editor(Rule {
                        sound,
                        ..Rule::default()
                    });
                }
            }
            60 => {
                self.activity.clear();
                self.refresh_activity();
            }
            70 => open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            ),
            71 => open_url("x-apple.systempreferences:com.apple.Notifications-Settings.extension"),
            72 => {
                let value: isize = msg_send![sender, state];
                let mut s = self.settings.clone();
                s.show_notification_text = value == 1;
                if self.commit(s) && !self.settings.show_notification_text {
                    for a in &mut self.activity {
                        a.detail.clear();
                    }
                }
                self.render();
            }
            73 => {
                let value: isize = msg_send![sender, state];
                if let Err(e) = set_login(value == 1) {
                    alert("Login item needs attention", &e);
                }
                self.render();
            }
            74 => {
                let value: f64 = msg_send![sender, doubleValue];
                let mut s = self.settings.clone();
                s.master_volume = value as f32;
                self.commit(s);
            }
            80 => match &self.updater {
                Ok(updater) => updater.check(),
                Err(error) => alert("Updates unavailable", error),
            },
            81 | 82 => {
                let value: isize = msg_send![sender, state];
                if let Ok(updater) = &self.updater {
                    if tag == 81 {
                        updater.set_checks(value == 1);
                    } else {
                        updater.set_downloads(value == 1 && updater.checks());
                    }
                }
                self.render();
            }
            1000..=1999 => {
                if let Some(rule) = self.settings.rules.get((tag - 1000) as usize) {
                    self.selected = Some(rule.id);
                    self.render();
                }
            }
            2000..=2999 => {
                if let Some((_, sound)) = self.sounds.get((tag - 2000) as usize).cloned() {
                    if let Err(e) = self.play(&sound, 0.7) {
                        alert("Could not play sound", &e);
                    }
                }
            }
            3000..=3999 => {
                if let Some((name, sound)) = self.sounds.get((tag - 3000) as usize).cloned() {
                    if self.settings.rules.iter().any(|r| r.sound == sound) {
                        alert(
                            "This sound is in use",
                            "Choose another sound in the rules that use it before deleting it.",
                        );
                        return;
                    }
                    if matches!(sound, Sound::Imported(_))
                        && confirm("Delete imported sound?", &name)
                    {
                        self.stop_audio();
                        match self
                            .storage
                            .sound_path(&sound)
                            .and_then(|p| std::fs::remove_file(p).map_err(|e| e.to_string()))
                        {
                            Ok(()) => {
                                self.sounds = self.storage.sounds();
                                self.render();
                            }
                            Err(e) => alert("Could not delete sound", &e),
                        }
                    }
                }
            }
            _ => {}
        }
    }
    unsafe fn stop_audio(&mut self) {
        if let Some(label) = &self.now_playing {
            set_text(label, "Press play to find your next notification sound.");
        }
        if let Some(player) = self.player.take() {
            let _: Bool = msg_send![&player, stop];
        }
    }
    unsafe fn play(&mut self, sound: &Sound, volume: f32) -> Result<(), String> {
        let path = self.storage.sound_path(sound)?;
        let player =
            load_audio(&path).ok_or_else(|| format!("Cannot open audio: {}", path.display()))?;
        self.stop_audio();
        let _: () = msg_send![&player,setVolume:(volume*self.settings.master_volume).clamp(0.,1.)];
        let played: Bool = msg_send![&player, play];
        if !played.as_bool() {
            return Err("macOS could not start audio playback.".into());
        }
        self.player = Some(player);
        if let Some(label) = &self.now_playing {
            let name = self
                .sounds
                .iter()
                .find(|(_, s)| s == sound)
                .map(|(n, _)| n.as_str())
                .unwrap_or("Sound");
            set_text(label, &format!("Playing  ·  {name}"));
        }
        Ok(())
    }
    unsafe fn update_status(&self) {
        let prefix = if self.settings.paused { "Paused\n" } else { "" };
        set_text(&self.status_label, &format!("{prefix}{}", self.status));
    }
    unsafe fn tick(&mut self) {
        if let Some(player) = &self.player {
            let playing: Bool = msg_send![player, isPlaying];
            if !playing.as_bool() {
                self.stop_audio();
            }
        }
        for _ in 0..32 {
            let Ok(message) = self.receiver.try_recv() else {
                break;
            };
            match message {
                Message::Status(status, trusted) => {
                    self.status = status;
                    self.trusted = trusted;
                    self.update_status();
                }
                Message::Event(event) => {
                    let decision = self.engine.evaluate(&event, &self.settings, Instant::now());
                    let outcome = match decision {
                        Decision::Paused => "Skipped · listening is paused".into(),
                        Decision::NoMatch => "No matching rule".into(),
                        Decision::Cooldown(_) => "Skipped · rule cooldown".into(),
                        Decision::Play(id) => {
                            if let Some(rule) =
                                self.settings.rules.iter().find(|r| r.id == id).cloned()
                            {
                                match self.play(&rule.sound, rule.volume) {
                                    Ok(()) => format!("Played · {}", rule.name),
                                    Err(e) => format!("Playback failed · {e}"),
                                }
                            } else {
                                "Rule no longer exists".into()
                            }
                        }
                    };
                    self.record(&event, outcome);
                }
            }
        }
    }
    unsafe fn record(&mut self, event: &Notification, outcome: String) {
        let formatter = new(class!(NSDateFormatter));
        let _: () = msg_send![&formatter,setDateFormat:&*string("HH:mm:ss")];
        let now: *mut AnyObject = msg_send![class!(NSDate), date];
        let time: Retained<NSString> = msg_send![&formatter,stringFromDate:now];
        self.activity.insert(
            0,
            Activity {
                app: event.app_name.clone(),
                detail: if self.settings.show_notification_text {
                    event.text.chars().take(2000).collect()
                } else {
                    String::new()
                },
                outcome,
                time: time.to_string(),
            },
        );
        self.activity.truncate(60);
        self.refresh_activity();
    }
}
unsafe fn scroll(parent: &AnyObject, r: NSRect, height: f64) -> Obj {
    let scroll = frame(class!(NSScrollView), r);
    let _: () = msg_send![&scroll,setHasVerticalScroller:Bool::YES];
    let _: () = msg_send![&scroll,setDrawsBackground:Bool::NO];
    let document = frame(
        class!(NSView),
        rect(0., 0., r.size.width - 18., height.max(r.size.height)),
    );
    let _: () = msg_send![&scroll,setDocumentView:&*document];
    add(parent, &scroll);
    let _: () =
        msg_send![&document,scrollPoint:NSPoint::new(0.,height.max(r.size.height)-r.size.height)];
    document
}
unsafe fn choose_file(title: &str, prompt: &str, directory: Option<&str>) -> Option<PathBuf> {
    let panel: Obj = msg_send![class!(NSOpenPanel), openPanel];
    let _: () = msg_send![&panel,setTitle:&*string(title)];
    let _: () = msg_send![&panel,setPrompt:&*string(prompt)];
    let _: () = msg_send![&panel,setCanChooseFiles:Bool::YES];
    let _: () = msg_send![&panel,setCanChooseDirectories:Bool::NO];
    let _: () = msg_send![&panel,setAllowsMultipleSelection:Bool::NO];
    if let Some(directory) = directory {
        let url: *mut AnyObject = msg_send![class!(NSURL),fileURLWithPath:&*string(directory)];
        let _: () = msg_send![&panel,setDirectoryURL:url];
    }
    let response: isize = msg_send![&panel, runModal];
    if response != 1 {
        return None;
    }
    let url: *mut AnyObject = msg_send![&panel, URL];
    if url.is_null() {
        return None;
    }
    let path: Retained<NSString> = msg_send![url, path];
    Some(PathBuf::from(path.to_string()))
}
unsafe fn load_audio(path: &std::path::Path) -> Option<Obj> {
    let allocated: Allocated<AnyObject> = msg_send![class!(NSSound), alloc];
    msg_send![allocated,initWithContentsOfFile:&*string(&path.to_string_lossy()),byReference:Bool::NO]
}
unsafe fn login_enabled() -> bool {
    let service: *mut AnyObject = msg_send![class!(SMAppService), mainAppService];
    let status: isize = msg_send![service, status];
    status == 1
}
unsafe fn set_login(enable: bool) -> Result<(), String> {
    let service: *mut AnyObject = msg_send![class!(SMAppService), mainAppService];
    let mut error: *mut AnyObject = ptr::null_mut();
    let success: Bool = if enable {
        msg_send![service,registerAndReturnError:&mut error]
    } else {
        msg_send![service,unregisterAndReturnError:&mut error]
    };
    if success.as_bool() {
        Ok(())
    } else if !error.is_null() {
        let text: Retained<NSString> = msg_send![error, localizedDescription];
        Err(text.to_string())
    } else {
        Err("Move Dunkerfy.app into Applications and try again.".into())
    }
}

/// Draw the app's original vector mark with AppKit at Retina icon resolution.
// Render actual AppKit views with synthetic data for layout QA, even on a locked Mac.
unsafe fn render_view(view: &AnyObject, path: &str) {
    let bounds: NSRect = msg_send![view, bounds];
    let _: () = msg_send![view, layoutSubtreeIfNeeded];
    let bitmap: Obj = msg_send![view,bitmapImageRepForCachingDisplayInRect:bounds];
    let _: () = msg_send![view,cacheDisplayInRect:bounds,toBitmapImageRep:&*bitmap];
    // NSWindow draws the background outside its content view. Composite that
    // background explicitly when rendering the view without a visible window.
    let alloc: Allocated<AnyObject> = msg_send![class!(NSImage), alloc];
    let image: Obj = msg_send![alloc,initWithSize:bounds.size];
    let _: () = msg_send![&image, lockFocus];
    let background: *mut AnyObject = msg_send![class!(NSColor), windowBackgroundColor];
    let _: () = msg_send![background, setFill];
    let _: () = msg_send![class!(NSBezierPath),fillRect:bounds];
    let _: Bool = msg_send![&bitmap,drawInRect:bounds];
    let _: () = msg_send![&image, unlockFocus];
    let tiff: *mut AnyObject = msg_send![&image, TIFFRepresentation];
    let composite: *mut AnyObject = msg_send![class!(NSBitmapImageRep),imageRepWithData:tiff];
    let properties = new(class!(NSDictionary));
    let png: *mut AnyObject =
        msg_send![composite,representationUsingType:4usize,properties:&*properties];
    let ok: Bool = msg_send![png,writeToFile:&*string(path),atomically:Bool::YES];
    assert!(ok.as_bool(), "Could not render native UI");
}

unsafe fn write_icon(path: &str) {
    let allocated: Allocated<AnyObject> = msg_send![class!(NSImage), alloc];
    let image: Obj = msg_send![allocated, initWithSize: NSSize::new(1024., 1024.)];
    let _: () = msg_send![&image, lockFocus];
    let background: *mut AnyObject = msg_send![class!(NSColor), colorWithSRGBRed: 0.055f64, green: 0.24f64, blue: 0.26f64, alpha: 1.0f64];
    let _: () = msg_send![background, setFill];
    let shape: *mut AnyObject = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: rect(64.,64.,896.,896.), xRadius: 210f64, yRadius: 210f64];
    let _: () = msg_send![shape, fill];
    let foreground: *mut AnyObject = msg_send![class!(NSColor), colorWithSRGBRed: 0.7f64, green: 0.98f64, blue: 0.87f64, alpha: 1.0f64];
    let _: () = msg_send![foreground, setFill];
    for (index, height) in [150., 300., 460., 300., 150.].iter().enumerate() {
        let bar: *mut AnyObject = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: rect(236.+index as f64*112.,512.-height/2.,72.,*height), xRadius:36f64, yRadius:36f64];
        let _: () = msg_send![bar, fill];
    }
    let accent: *mut AnyObject = msg_send![class!(NSColor), colorWithSRGBRed: 1f64, green: 0.68f64, blue: 0.38f64, alpha: 1.0f64];
    let _: () = msg_send![accent, setFill];
    let dot: *mut AnyObject =
        msg_send![class!(NSBezierPath), bezierPathWithOvalInRect: rect(754.,744.,108.,108.)];
    let _: () = msg_send![dot, fill];
    let _: () = msg_send![&image, unlockFocus];
    let tiff: *mut AnyObject = msg_send![&image, TIFFRepresentation];
    let bitmap: *mut AnyObject = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
    let properties = new(class!(NSDictionary));
    let png: *mut AnyObject =
        msg_send![bitmap, representationUsingType: 4usize, properties: &*properties];
    let result: Bool = msg_send![png, writeToFile: &*string(path), atomically: Bool::YES];
    assert!(result.as_bool(), "Could not write app icon");
}
