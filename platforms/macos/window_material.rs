//! An AppKit backdrop for the existing NSH window content hierarchy.

use std::cell::{Cell, RefCell};
use std::ptr::{self, NonNull};
use std::rc::Rc;

use block2::RcBlock;
use nsh_core::JsonValue;
use nsh_platform_macos::objc2::rc::{Retained, Weak};
use nsh_platform_macos::objc2::runtime::{AnyObject, ProtocolObject};
use nsh_platform_macos::objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use nsh_platform_macos::objc2_app_kit::{
    NSAppearanceCustomization, NSAutoresizingMaskOptions, NSColor,
    NSUserInterfaceItemIdentification, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowOrderingMode, NSWorkspace,
    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use nsh_platform_macos::objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue, NSPoint, NSRect,
    ns_string,
};
use nsh_platform_macos::{ComponentContext, Components, NativeComponent};

pub(crate) fn register(components: &Components) {
    components.register(
        "UsherMacWindowMaterial",
        Rc::new(|context| Box::new(WindowMaterial(MaterialMarker::new(context)))),
    );
}

define_class!(
    #[unsafe(super(NSVisualEffectView, NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "UsherWindowBackdrop"]
    struct Backdrop;

    impl Backdrop {
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> Option<Retained<NSView>> {
            None
        }
    }
);

/// Holds only the state the backdrop changed. The content view still belongs to NSH.
struct Attachment {
    window: Weak<NSWindow>,
    effect: Retained<Backdrop>,
    previous_color: Retained<NSColor>,
    previous_opaque: bool,
    applied_color: Retained<NSColor>,
}

impl Attachment {
    fn install(window: &NSWindow) -> Option<Self> {
        let content = window.contentView()?;
        let mtm = window.mtm();
        let effect: Retained<Backdrop> =
            // SAFETY: initializes the NSVisualEffectView subclass on the main thread.
            unsafe { msg_send![Backdrop::alloc(mtm), initWithFrame: content.bounds()] };
        effect.setIdentifier(Some(ns_string!("usher.window.material")));
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        effect.setMaterial(NSVisualEffectMaterial::Sidebar);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::FollowsWindowActiveState);

        // Preserve the native clipping/shadow that a fully clear window can lose.
        let applied_color = NSColor::colorWithSRGBRed_green_blue_alpha(0.5, 0.5, 0.5, 0.001);
        let attachment = Self {
            window: Weak::new(window),
            effect,
            previous_color: window.backgroundColor(),
            previous_opaque: window.isOpaque(),
            applied_color,
        };
        // NSH's render root stays where it is, above this underlay. Installing
        // the effect before changing window opacity avoids a bare frame.
        content.addSubview_positioned_relativeTo(
            &attachment.effect,
            NSWindowOrderingMode::Below,
            None,
        );
        window.setBackgroundColor(Some(&attachment.applied_color));
        window.setOpaque(false);
        Some(attachment)
    }

    fn belongs_to(&self, window: &NSWindow) -> bool {
        let effect: &NSView = &self.effect;
        self.window
            .load()
            .is_some_and(|host| ptr::eq(&*host, window))
            && window.contentView().is_some_and(|content| {
                content
                    .subviews()
                    .iter()
                    .any(|view| ptr::eq(&*view, effect))
            })
    }

    fn already_present(window: &NSWindow) -> bool {
        window.contentView().is_some_and(|content| {
            content.subviews().iter().any(|view| {
                view.identifier().as_deref() == Some(ns_string!("usher.window.material"))
            })
        })
    }
}

impl Drop for Attachment {
    fn drop(&mut self) {
        if let Some(window) = self.window.load()
            && window.backgroundColor() == self.applied_color
        {
            // A later change by the window owner takes precedence.
            window.setBackgroundColor(Some(&self.previous_color));
            if !window.isOpaque() {
                window.setOpaque(self.previous_opaque);
            }
        }
        self.effect.removeFromSuperview();
    }
}

struct DisplayOptionsObserver {
    center: Retained<NSNotificationCenter>,
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl Drop for DisplayOptionsObserver {
    fn drop(&mut self) {
        // SAFETY: the token was returned by this notification center.
        unsafe { self.center.removeObserver(self.token.as_ref()) }
    }
}

struct MarkerState {
    context: ComponentContext,
    active: Cell<bool>,
    scheduled: Cell<bool>,
    waited_for_detach: Cell<bool>,
    reported_ready: Cell<Option<bool>>,
    attachment: RefCell<Option<Attachment>>,
    observer: RefCell<Option<DisplayOptionsObserver>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "UsherWindowMaterialMarker"]
    #[ivars = MarkerState]
    struct MaterialMarker;

    impl MaterialMarker {
        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            // SAFETY: forward the NSView lifecycle callback to its superclass.
            unsafe { msg_send![super(self), viewDidMoveToWindow] }
            self.schedule_sync();
        }

        #[unsafe(method(viewDidChangeEffectiveAppearance))]
        fn appearance_changed(&self) {
            // SAFETY: forward the NSView lifecycle callback to its superclass.
            unsafe { msg_send![super(self), viewDidChangeEffectiveAppearance] }
            self.schedule_sync();
        }

        #[unsafe(method(usherSyncWindowMaterial:))]
        fn sync_material(&self, _argument: Option<&AnyObject>) {
            self.ivars().scheduled.set(false);
            self.sync();
        }
    }
);

impl MaterialMarker {
    fn new(context: ComponentContext) -> Retained<Self> {
        let mtm =
            MainThreadMarker::new().expect("NSH creates AppKit components on the main thread");
        let marker = Self::alloc(mtm).set_ivars(MarkerState {
            context,
            active: Cell::new(true),
            scheduled: Cell::default(),
            waited_for_detach: Cell::default(),
            reported_ready: Cell::default(),
            attachment: RefCell::default(),
            observer: RefCell::default(),
        });
        // SAFETY: initialize the marker as an NSView after setting its ivars.
        let marker: Retained<Self> =
            unsafe { msg_send![super(marker), initWithFrame: NSRect::ZERO] };
        marker.observe_display_options();
        marker
    }

    fn observe_display_options(&self) {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let queue = NSOperationQueue::mainQueue();
        let marker = Weak::new(self);
        let block = RcBlock::new(move |_: NonNull<NSNotification>| {
            if let Some(marker) = marker.load() {
                marker.schedule_sync();
            }
        });
        // SAFETY: the name is a static AppKit constant. Delivery on the main
        // queue keeps this weak reference and the view on their owning thread.
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
                Some(&queue),
                &block,
            )
        };
        *self.ivars().observer.borrow_mut() = Some(DisplayOptionsObserver { center, token });
    }

    fn schedule_sync(&self) {
        if self.ivars().scheduled.replace(true) {
            return;
        }
        // Changing native siblings during an NSH mount can resize the render
        // root before that mount returns. Defer to AppKit's next run-loop turn.
        // SAFETY: this view implements the void selector with one object
        // argument. The run loop retains the receiver until it runs.
        unsafe {
            let _: () = msg_send![
                self,
                performSelector: sel!(usherSyncWindowMaterial:),
                withObject: Option::<&AnyObject>::None,
                afterDelay: 0.0_f64
            ];
        }
    }

    fn sync(&self) {
        let state = self.ivars();
        if state.active.get()
            && let Some(window) = self.window()
        {
            super::window_geometry::configure(&window);
        }
        let workspace = NSWorkspace::sharedWorkspace();
        let window = if state.active.get()
            && !workspace.accessibilityDisplayShouldReduceTransparency()
            && !workspace.accessibilityDisplayShouldIncreaseContrast()
        {
            self.window()
        } else {
            None
        };
        let attached = state
            .attachment
            .borrow()
            .as_ref()
            .is_some_and(|attachment| {
                window
                    .as_deref()
                    .is_some_and(|window| attachment.belongs_to(window))
            });
        if !attached {
            let previous = state.attachment.borrow_mut().take();
            drop(previous);
            if let Some(window) = window.as_deref() {
                // A remount can arrive before the old marker's deferred
                // detach. Only one marker may own the window's saved state.
                if Attachment::already_present(window) {
                    if !state.waited_for_detach.replace(true) {
                        self.schedule_sync();
                        return;
                    }
                } else {
                    state.waited_for_detach.set(false);
                    *state.attachment.borrow_mut() = Attachment::install(window);
                }
            }
        }
        let attachment = state.attachment.borrow();
        if let Some(attachment) = attachment.as_ref() {
            // NSH applies an explicit light/dark override to its root view.
            // Its native sibling needs the same appearance without changing the window.
            attachment
                .effect
                .setAppearance(Some(&self.effectiveAppearance()));
        }
        let installed = attachment.is_some();
        if state.active.get() && state.reported_ready.replace(Some(installed)) != Some(installed) {
            state.context.emit(
                "ready",
                JsonValue::Object(
                    [("installed".into(), installed.into())]
                        .into_iter()
                        .collect(),
                ),
            );
        }
    }
}

struct WindowMaterial(Retained<MaterialMarker>);

impl NativeComponent for WindowMaterial {
    fn view(&self) -> Retained<NSView> {
        Retained::into_super(self.0.clone())
    }

    fn set_prop(&mut self, _name: &str, _value: Option<&JsonValue>) {}
}

impl Drop for WindowMaterial {
    fn drop(&mut self) {
        let state = self.0.ivars();
        state.active.set(false);
        state.observer.borrow_mut().take();
        self.0.schedule_sync();
    }
}
