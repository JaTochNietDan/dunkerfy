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

pub fn run() {
    autoreleasepool(|_| unsafe {
        if !std::env::args().any(|arg| arg == "--smoke-test" || arg == "--write-icon") {
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
        let target = new(builder.register());
        let _: () = msg_send![&application, setDelegate: &*target];
        let storage = match Storage::standard() {
            Ok(s) => s,
            Err(e) => {
                alert("Cannot start Dunkerfy", &e);
                return;
            }
        };
        let settings = match storage.load() {
            Ok(s) => s,
            Err(e) => {
                alert("Settings need attention", &e);
                return;
            }
        };
        let alloc: Allocated<AnyObject> = msg_send![class!(NSWindow), alloc];
        let window: Obj = msg_send![alloc, initWithContentRect: rect(0.,0.,1040.,720.), styleMask: 7usize, backing: 2usize, defer: Bool::NO];
        let _: () = msg_send![&window, setTitle: &*string("Dunkerfy")];
        let _: () = msg_send![&window, setReleasedWhenClosed: Bool::NO];
        let _: () = msg_send![&window, center];
        let root: *mut AnyObject = msg_send![&window, contentView];
        let root = &*root;
        let sidebar = frame(class!(NSVisualEffectView), rect(0., 0., 210., 720.));
        let _: () = msg_send![&sidebar, setMaterial: 7isize];
        let _: () = msg_send![&sidebar, setBlendingMode: 0isize];
        add(root, &sidebar);
        label(
            &sidebar,
            "◉  Dunkerfy",
            rect(22., 650., 178., 36.),
            25.,
            true,
        );
        secondary(&label(
            &sidebar,
            "A sound for every app.",
            rect(24., 619., 175., 25.),
            12.,
            false,
        ));
        for (i, title) in ["Sound rules", "Sound library", "Activity", "Settings"]
            .iter()
            .enumerate()
        {
            button(
                &sidebar,
                &target,
                title,
                10 + i as isize,
                rect(18., 550. - i as f64 * 46., 174., 34.),
            );
        }
        label(
            &sidebar,
            "ON THIS MAC",
            rect(24., 172., 170., 20.),
            10.,
            true,
        );
        let status_label = label(
            &sidebar,
            "Starting…",
            rect(24., 107., 166., 58.),
            12.,
            false,
        );
        secondary(&status_label);
        let pause_button = button(
            &sidebar,
            &target,
            if settings.paused {
                "Resume listening"
            } else {
                "Pause listening"
            },
            20,
            rect(18., 49., 174., 32.),
        );
        let content = frame(class!(NSView), rect(210., 0., 830., 720.));
        add(root, &content);
        let menu = new(class!(NSMenu));
        menu_item(&menu, &target, "Open Dunkerfy", 1, "");
        let menu_pause = menu_item(
            &menu,
            &target,
            if settings.paused { "Resume" } else { "Pause" },
            20,
            "",
        );
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
            receiver: monitor::start(),
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
        if std::env::args().any(|arg| arg == "--smoke-test") {
            for page in 0..4 {
                app.page = page;
                app.render();
            }
            app.page = 0;
            app.clear_page();
            app.draw_editor(Rule::default());
            println!(
                "Native AppKit smoke test passed: four pages, editor, menu, and login service."
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
        self.editor = None;
        self.activity_view = None;
        let subviews: Obj = msg_send![&self.content, subviews];
        let copy: Obj = msg_send![&subviews, copy];
        let count: usize = msg_send![&copy, count];
        for i in 0..count {
            let v: *mut AnyObject = msg_send![&copy, objectAtIndex: i];
            let _: () = msg_send![v, removeFromSuperview];
        }
    }
    unsafe fn header(&self, title: &str, subtitle: &str) {
        label(&self.content, title, rect(32., 641., 760., 40.), 29., true);
        secondary(&label(
            &self.content,
            subtitle,
            rect(32., 595., 760., 38.),
            13.,
            false,
        ));
    }
    unsafe fn render(&mut self) {
        self.clear_page();
        match self.page {
            0 => self.rules_page(),
            1 => self.sounds_page(),
            2 => self.activity_page(),
            _ => self.settings_page(),
        }
    }
    unsafe fn rules_page(&mut self) {
        self.header(
            "Make every notification recognizable.",
            "Choose an app. Give it a sound. Add a keyword when you need something more specific.",
        );
        button(
            &self.content,
            &self.target,
            "+ New rule",
            30,
            rect(30., 547., 130., 32.),
        );
        label(
            &self.content,
            "FIRST MATCH WINS",
            rect(34., 516., 230., 19.),
            10.,
            true,
        );
        let list = scroll(
            &self.content,
            rect(30., 95., 260., 411.),
            self.settings.rules.len().max(1) as f64 * 64.,
        );
        let height = (self.settings.rules.len().max(1) as f64 * 64.).max(411.);
        if self.settings.rules.is_empty() {
            secondary(&label(
                &list,
                "Your apps, your soundtrack.\nAdd your first rule to get started.",
                rect(10., 5., 236., 52.),
                13.,
                false,
            ));
        }
        for (i, rule) in self.settings.rules.iter().enumerate() {
            let prefix = if Some(rule.id) == self.selected {
                "▸ "
            } else if rule.enabled {
                ""
            } else {
                "○ "
            };
            button(
                &list,
                &self.target,
                &format!("{prefix}{}", rule.name),
                1000 + i as isize,
                rect(4., height - (i + 1) as f64 * 64. + 27., 246., 30.),
            );
            secondary(&label(
                &list,
                &format!(
                    "{}  ·  {}",
                    rule.app_name,
                    if rule.contains.is_empty() {
                        "All notifications"
                    } else {
                        &rule.contains
                    }
                ),
                rect(12., height - (i + 1) as f64 * 64. + 4., 235., 20.),
                11.,
                false,
            ));
        }
        button(
            &self.content,
            &self.target,
            "Move up",
            33,
            rect(30., 46., 122., 30.),
        );
        button(
            &self.content,
            &self.target,
            "Move down",
            34,
            rect(160., 46., 130., 30.),
        );
        let rule = self
            .selected
            .and_then(|id| self.settings.rules.iter().find(|r| r.id == id).cloned());
        if let Some(rule) = rule {
            self.draw_editor(rule);
        } else {
            label(
                &self.content,
                "A little more signal.\nA lot less guessing.",
                rect(350., 344., 420., 104.),
                28.,
                true,
            );
            secondary(&label(
                &self.content,
                "Create a rule to hear which app needs you, without reaching for your screen.",
                rect(350., 260., 390., 64.),
                15.,
                false,
            ));
            button(
                &self.content,
                &self.target,
                "Create your first rule",
                30,
                rect(347., 205., 205., 36.),
            );
        }
    }
    unsafe fn draw_editor(&mut self, rule: Rule) {
        label(
            &self.content,
            "RULE DETAILS",
            rect(325., 553., 400., 20.),
            10.,
            true,
        );
        label(
            &self.content,
            "Name",
            rect(326., 519., 430., 20.),
            12.,
            true,
        );
        let name = entry(
            &self.content,
            &rule.name,
            "e.g. Work messages",
            rect(325., 483., 463., 28.),
        );
        label(
            &self.content,
            "Application",
            rect(326., 447., 430., 20.),
            12.,
            true,
        );
        let app = entry(
            &self.content,
            &rule.app_name,
            "Exact name shown on the notification",
            rect(325., 413., 321., 28.),
        );
        button(
            &self.content,
            &self.target,
            "Choose app…",
            40,
            rect(655., 411., 134., 32.),
        );
        label(
            &self.content,
            "Notification contains  ·  optional",
            rect(326., 376., 450., 20.),
            12.,
            true,
        );
        let contains = entry(
            &self.content,
            &rule.contains,
            "Person, channel, or keyword",
            rect(325., 341., 463., 28.),
        );
        label(
            &self.content,
            "Sound",
            rect(326., 305., 430., 20.),
            12.,
            true,
        );
        let popup = frame(class!(NSPopUpButton), rect(322., 268., 325., 30.));
        let mut found = false;
        for (i, (name, sound)) in self.sounds.iter().enumerate() {
            let _: () = msg_send![&popup, addItemWithTitle: &*string(name)];
            if *sound == rule.sound {
                let _: () = msg_send![&popup, selectItemAtIndex: i as isize];
                found = true;
            }
        }
        if !found {
            self.sounds
                .push(("Missing sound — choose another".into(), rule.sound.clone()));
            let _: () =
                msg_send![&popup, addItemWithTitle: &*string("Missing sound — choose another")];
            let _: () = msg_send![&popup, selectItemAtIndex: (self.sounds.len()-1) as isize];
        }
        add(&self.content, &popup);
        button(
            &self.content,
            &self.target,
            "▶ Preview",
            41,
            rect(657., 267., 132., 32.),
        );
        label(
            &self.content,
            "Volume",
            rect(326., 230., 150., 20.),
            12.,
            true,
        );
        let volume = slider(
            &self.content,
            &self.target,
            rule.volume,
            49,
            rect(325., 202., 300., 24.),
        );
        let enabled = check(
            &self.content,
            &self.target,
            "Enabled",
            49,
            rule.enabled,
            rect(665., 200., 120., 25.),
        );
        label(
            &self.content,
            "Cooldown (seconds)",
            rect(326., 163., 190., 22.),
            12.,
            true,
        );
        let cooldown = entry(
            &self.content,
            &rule.cooldown_secs.to_string(),
            "1",
            rect(530., 160., 75., 27.),
        );
        secondary(&label(
            &self.content,
            "Limits repeated sounds from this rule.",
            rect(326., 129., 455., 22.),
            11.,
            false,
        ));
        button(
            &self.content,
            &self.target,
            "Delete",
            32,
            rect(323., 64., 100., 32.),
        );
        button(
            &self.content,
            &self.target,
            "Test rule",
            42,
            rect(535., 64., 118., 32.),
        );
        let save = button(
            &self.content,
            &self.target,
            "Save rule",
            31,
            rect(665., 64., 124., 32.),
        );
        let _: () = msg_send![&save, setKeyEquivalent: &*string("\r")];
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
    unsafe fn sounds_page(&self) {
        self.header("Find your sound.","Built-in macOS sounds and your own audio. Imported files are copied into Dunkerfy’s library.");
        button(
            &self.content,
            &self.target,
            "Import audio…",
            50,
            rect(30., 547., 160., 32.),
        );
        secondary(&label(
            &self.content,
            "AIFF, WAV, MP3, M4A, CAF · up to 20 MB",
            rect(212., 550., 550., 25.),
            12.,
            false,
        ));
        let height = (self.sounds.len() as f64 * 49.).max(475.);
        let list = scroll(&self.content, rect(30., 40., 760., 475.), height);
        for (i, (name, sound)) in self.sounds.iter().enumerate() {
            label(
                &list,
                name,
                rect(16., height - (i + 1) as f64 * 49. + 10., 475., 25.),
                14.,
                false,
            );
            button(
                &list,
                &self.target,
                "▶ Preview",
                2000 + i as isize,
                rect(503., height - (i + 1) as f64 * 49. + 7., 116., 32.),
            );
            if matches!(sound, Sound::Imported(_)) {
                button(
                    &list,
                    &self.target,
                    "Delete",
                    3000 + i as isize,
                    rect(631., height - (i + 1) as f64 * 49. + 7., 92., 32.),
                );
            }
        }
    }
    unsafe fn activity_page(&mut self) {
        self.header("Know what Dunkerfy heard.","The last 60 events, kept only in memory. Notification text is hidden unless you enable it in Settings.");
        button(
            &self.content,
            &self.target,
            "Clear activity",
            60,
            rect(30., 547., 150., 32.),
        );
        let scroll = frame(class!(NSScrollView), rect(30., 40., 760., 478.));
        let _: () = msg_send![&scroll,setHasVerticalScroller:Bool::YES];
        let text = frame(class!(NSTextView), rect(0., 0., 740., 478.));
        let _: () = msg_send![&text,setEditable:Bool::NO];
        let _: () = msg_send![&text,setVerticallyResizable:Bool::YES];
        let _: () = msg_send![&text,setAutoresizingMask:2usize];
        let _: () = msg_send![&text,setTextContainerInset:NSSize::new(16.,16.)];
        let font: *mut AnyObject = msg_send![class!(NSFont),systemFontOfSize:13f64];
        let _: () = msg_send![&text,setFont:font];
        let color: *mut AnyObject = msg_send![class!(NSColor), labelColor];
        let _: () = msg_send![&text,setTextColor:color];
        let _: () = msg_send![&scroll,setDocumentView:&*text];
        add(&self.content, &scroll);
        self.activity_view = Some(text);
        self.refresh_activity();
    }
    unsafe fn refresh_activity(&self) {
        if let Some(view) = &self.activity_view {
            let text = if self.activity.is_empty() {
                "No notifications observed yet.\n\nEnable Accessibility access, then trigger a notification from an app with banners enabled.".into()
            } else {
                self.activity
                    .iter()
                    .map(|a| {
                        format!(
                            "{}  ·  {}\n{}{}\n",
                            a.time,
                            a.app,
                            a.outcome,
                            if a.detail.is_empty() {
                                String::new()
                            } else {
                                format!("\n{}", a.detail)
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let _: () = msg_send![&**view,setString:&*string(&text)];
        }
    }
    unsafe fn settings_page(&self) {
        self.header("Quietly working for you.","Dunkerfy runs locally in your menu bar. No account, analytics, or network connection required.");
        label(
            &self.content,
            "01   Allow Accessibility access",
            rect(32., 540., 745., 28.),
            18.,
            true,
        );
        secondary(&label(&self.content,"Dunkerfy reads visible notification banners to identify their app and match your rules. Enable Dunkerfy in System Settings → Privacy & Security → Accessibility.",rect(32.,475.,735.,54.),13.,false));
        button(
            &self.content,
            &self.target,
            "Open Accessibility Settings",
            70,
            rect(29., 427., 250., 34.),
        );
        label(
            &self.content,
            "02   Turn off each app’s original sound",
            rect(32., 373., 745., 28.),
            18.,
            true,
        );
        secondary(&label(&self.content,"In Notifications settings, select an app and turn off “Play sound for notification.” Keep banners or alerts enabled. Dunkerfy plays the replacement; it cannot suppress the original sound.",rect(32.,305.,735.,58.),13.,false));
        button(
            &self.content,
            &self.target,
            "Open Notification Settings",
            71,
            rect(29., 259., 250., 34.),
        );
        check(
            &self.content,
            &self.target,
            "Show notification text in the in-memory activity log",
            72,
            self.settings.show_notification_text,
            rect(32., 207., 740., 26.),
        );
        check(
            &self.content,
            &self.target,
            "Launch Dunkerfy at login",
            73,
            login_enabled(),
            rect(32., 169., 740., 26.),
        );
        label(
            &self.content,
            "Master volume",
            rect(32., 121., 160., 24.),
            13.,
            true,
        );
        slider(
            &self.content,
            &self.target,
            self.settings.master_volume,
            74,
            rect(198., 120., 420., 25.),
        );
        secondary(&label(&self.content,"Focus and hidden previews can limit which notifications are available. Closing this window keeps Dunkerfy listening; choose Quit from the menu bar to stop.",rect(32.,37.,735.,58.),12.,false));
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
        if matches!(tag, 2 | 10..=13 | 30 | 33..=34 | 1000..=1999) && !self.may_discard_edits() {
            return;
        }
        match tag {
            1 => self.show(),
            2 => {
                let _: () = msg_send![&self.application,terminate:ptr::null::<AnyObject>()];
            }
            10..=13 => {
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
                self.clear_page();
                self.header("Create a sound rule.","Changes are saved only when you choose Save rule. Choose the app’s exact notification name.");
                label(
                    &self.content,
                    "Start with one app.",
                    rect(32., 476., 260., 36.),
                    20.,
                    true,
                );
                secondary(&label(&self.content, "Choose an application, or type the exact app name from its notification banner.\n\nLeave the keyword empty to match every notification from that app.\n\nMore specific rules should go above general rules.", rect(32., 260., 250., 205.), 14., false));
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
        Ok(())
    }
    unsafe fn update_status(&self) {
        let prefix = if self.settings.paused { "Paused\n" } else { "" };
        set_text(&self.status_label, &format!("{prefix}{}", self.status));
    }
    unsafe fn tick(&mut self) {
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
