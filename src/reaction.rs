//! ReAction windows and gadgets, safe (feature `reaction`, with
//! `intuition` and `listbrowser`).
//!
//! A window is described as a tree of [`Layout`]s and [`Gadget`]s, then
//! opened with [`Window::open`]: the crate makes the BOOPSI objects
//! (window.class, layout.gadget, and button, string, checkbox, integer,
//! slider, chooser, listbrowser and fuelgauge gadgets, label images),
//! opens the window and keeps everything the objects point at (texts,
//! label arrays, list browser node lists) for as long as they live. The
//! window object owns the layout, which owns the gadgets: dropping the
//! [`Window`] disposes the lot, then frees the lists.
//!
//! - Events: [`Window::next`] runs WM_HANDLEINPUT and returns one
//!   [`Event`] (a gadget released with its id and code, the close gadget,
//!   a key, a resize); [`Window::signal_mask`] to Wait on.
//! - Values by gadget id: [`Window::text`], [`checked`](Window::checked),
//!   [`number`](Window::number), [`level`](Window::level),
//!   [`active`](Window::active), [`selected`](Window::selected) and their
//!   setters, [`set_disabled`](Window::set_disabled),
//!   [`set_items`](Window::set_items) (a list browser's rows).
//!
//! The classes are opened (OpenClass) the first time they are needed and
//! kept for the rest of the program, like the crate's interfaces.
//!
//! ```ignore
//! let root = Layout::vertical()
//!     .child(Gadget::string(1).text("Amiga").label("Name"))
//!     .child(Gadget::checkbox(2, "Shout"))
//!     .child(Layout::horizontal().child(Gadget::button(3, "OK")).child(Gadget::button(4, "Cancel")));
//! let mut w = Window::open(&screen, &WindowSpec { title: "Hello", ..Default::default() }, root)?;
//! loop {
//!     amigaos4::signal::wait(w.signal_mask());
//!     while let Some(e) = w.next() {
//!         match e {
//!             Event::Close | Event::Gadget { id: 4, .. } => return Ok(()),
//!             Event::Gadget { id: 3, .. } => println!("{}", w.text(1).unwrap_or_default()),
//!             _ => {}
//!         }
//!     }
//! }
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::cstr::c_copy;
use crate::error::{AmigaError, Result};
use crate::intuition::PubScreen;
use crate::sys::{self, IntuitionIFace, TagItem, APTR, TAG_DONE};

crate::iface::iface!(lb_iface, sys::ListBrowserIFace, IListBrowser, b"gadgets/listbrowser.gadget\0", b"main\0", 50);
crate::iface::iface!(sb_iface, sys::SpeedBarIFace, ISpeedBar, b"gadgets/speedbar.gadget\0", b"main\0", 50);

// Tags, methods and results (classes/window.h, gadgets/*.h, images/*.h,
// intuition/*.h; tests/layout/reaction.c).
const TAG_USER: u32 = 0x8000_0000;
const WINDOW_SIG_MASK: u32 = TAG_USER + 0x0502_5002;
const WINDOW_LAYOUT: u32 = TAG_USER + 0x0502_5005;
const WINDOW_POSITION: u32 = TAG_USER + 0x0502_500e;
const WINDOW_MENU_STRIP: u32 = TAG_USER + 0x0502_5004;
const WINDOW_MENU_ADDRESS: u32 = TAG_USER + 0x0502_5041;
const WINDOW_MENU_TYPE: u32 = TAG_USER + 0x0502_5042;
const GA_CONTEXT_MENU: u32 = TAG_USER + 0x30039;
const IDCMP_MENUPICK: u32 = 0x100;
const WMHI_MENUPICK: u32 = 6 << 16;
const WPOS_CENTERSCREEN: u32 = 1;
const WPOS_CENTERMOUSE: u32 = 2;
const WINDOW_APP_PORT: u32 = TAG_USER + 0x0502_500d;
const WINDOW_ICON_TITLE: u32 = TAG_USER + 0x0502_5012;
const WINDOW_APP_MSG_HOOK: u32 = TAG_USER + 0x0502_5013;
const WINDOW_APP_WINDOW: u32 = TAG_USER + 0x0502_5015;
const WINDOW_ICONIFY_GADGET: u32 = TAG_USER + 0x0502_5017;
const WM_ICONIFY: u32 = 0x0057_0005;
const WINDOW_QUALIFIER: u32 = TAG_USER + 0x0502_5035;
const WMHI_ICONIFY: u32 = 9 << 16;
const WMHI_UNICONIFY: u32 = 10 << 16;
const SPACE_TRANSPARENT: u32 = TAG_USER + 0x0500_9005;
const SPEEDBAR_BUTTONS: u32 = TAG_USER + 0x0501_3001;
const SBNA_HELP: u32 = TAG_USER + 0x0001_000b;
const SBNA_IMAGE: u32 = TAG_USER + 0x0001_0009;
const SBNA_TEXT: u32 = TAG_USER + 0x0001_0010;
const SBNA_SEPARATOR: u32 = TAG_USER + 0x0001_0012;
const SBNA_BUTTON_ID: u32 = TAG_USER + 0x0001_0014;
const SBNA_HINT_INFO: u32 = TAG_USER + 0x0001_0013;
const SBNA_TOGGLE: u32 = TAG_USER + 0x0001_000c;
const GM_DOMAIN: u32 = 7;
const GDOMAIN_NOMINAL: i32 = 1;
/// struct Screen's RastPort (tests/layout/reaction.c).
const SCREEN_RASTPORT: usize = 84;

/// gpDomain (intuition/gadgetclass.h; tests/layout/reaction.c).
#[repr(C)]
struct GpDomain {
    method: u32,
    ginfo: APTR,
    rport: *mut sys::RastPort,
    which: i32,
    domain: [i16; 4],
    attrs: *mut TagItem,
}
const SBNA_SELECTED: u32 = TAG_USER + 0x0001_000d;
const GA_HINT_INFO: u32 = TAG_USER + 0x30038;
const WINDOW_GADGET_HELP: u32 = TAG_USER + 0x0502_5016;
const BITMAP_SOURCE_FILE: u32 = TAG_USER + 0x0501_9001;
const BITMAP_SCREEN: u32 = TAG_USER + 0x0501_9002;
const BITMAP_MASKING: u32 = TAG_USER + 0x0501_9004;
const SPACE_RENDER_BOX: u32 = TAG_USER + 0x0500_900a;
const WA_SCREEN_TITLE: u32 = TAG_USER + 0x6f;
const WA_SMART_REFRESH: u32 = TAG_USER + 141;
const WA_SIMPLE_REFRESH: u32 = TAG_USER + 140;
const WA_BUSY_POINTER: u32 = TAG_USER + 152;
const WA_POINTER_DELAY: u32 = TAG_USER + 153;
/// struct Window: LeftEdge 4, TopEdge 6, Width 8, Height 10, RPort 50
/// (tests/layout/intuition.c).
const WIN_LEFT: usize = 4;
#[cfg_attr(not(feature = "graphics"), allow(dead_code))]
const WIN_RPORT: usize = 50;
/// struct Window's BorderLeft, Top, Right, Bottom (BYTEs; tests/layout/intuition.c).
const WIN_BORDERS: usize = 54;
/// struct Window's MinWidth, MinHeight (tests/layout/reaction.c).
const WIN_MIN_SIZE: usize = 16;
const WM_HANDLEINPUT: u32 = 0x0057_0001;
const WM_OPEN: u32 = 0x0057_0002;
const LAYOUT_ORIENTATION: u32 = TAG_USER + 0x0500_7001;
const LAYOUT_EVEN_SIZE: u32 = TAG_USER + 0x0500_7007;
const LAYOUT_BEVEL_STYLE: u32 = TAG_USER + 0x0500_700f;
const LAYOUT_LABEL: u32 = TAG_USER + 0x0500_7010;
const LAYOUT_ADD_CHILD: u32 = TAG_USER + 0x0500_7014;
const LAYOUT_SPACE_INNER: u32 = TAG_USER + 0x0500_7025;
const LAYOUT_SPACE_OUTER: u32 = TAG_USER + 0x0500_7026;
const LAYOUT_ORIENT_HORIZ: u32 = 0;
const LAYOUT_ORIENT_VERT: u32 = 1;
const BVS_GROUP: u32 = 2;
const CHILD_MIN_WIDTH: u32 = TAG_USER + 0x0500_7101;
const CHILD_MIN_HEIGHT: u32 = TAG_USER + 0x0500_7102;
const CHILD_WEIGHTED_WIDTH: u32 = TAG_USER + 0x0500_7105;
const CHILD_WEIGHTED_HEIGHT: u32 = TAG_USER + 0x0500_7106;
const CHILD_LABEL: u32 = TAG_USER + 0x0500_710c;
const GA_TEXT: u32 = TAG_USER + 0x30009;
const GA_DISABLED: u32 = TAG_USER + 0x3000e;
const GA_ID: u32 = TAG_USER + 0x30010;
const GA_SELECTED: u32 = TAG_USER + 0x30013;
const GA_RELVERIFY: u32 = TAG_USER + 0x30016;
const GA_TAB_CYCLE: u32 = TAG_USER + 0x30024;
const BUTTON_PUSH_BUTTON: u32 = TAG_USER + 0x0400_0001;
const STRINGA_MAX_CHARS: u32 = TAG_USER + 0x32001;
const STRINGA_TEXT_VAL: u32 = TAG_USER + 0x32012;
const CHECKBOX_CHECKED: u32 = GA_SELECTED;
const INTEGER_NUMBER: u32 = TAG_USER + 0x0500_2001;
const INTEGER_MINIMUM: u32 = TAG_USER + 0x0500_2003;
const INTEGER_MAXIMUM: u32 = TAG_USER + 0x0500_2004;
const INTEGER_ARROWS: u32 = TAG_USER + 0x0500_2005;
const SLIDER_MIN: u32 = TAG_USER + 0x0502_8001;
const SLIDER_MAX: u32 = TAG_USER + 0x0502_8002;
const SLIDER_LEVEL: u32 = TAG_USER + 0x0502_8003;
const SLIDER_ORIENTATION: u32 = TAG_USER + 0x0502_8004;
const SORIENT_HORIZ: u32 = 2;
const SORIENT_VERT: u32 = 4;
const CHOOSER_POP_UP: u32 = TAG_USER + 0x0500_1001;
const CHOOSER_ACTIVE: u32 = TAG_USER + 0x0500_1005;
const CHOOSER_LABEL_ARRAY: u32 = TAG_USER + 0x0500_100c;
const LISTBROWSER_LABELS: u32 = TAG_USER + 0x0500_3003;
const LISTBROWSER_SELECTED: u32 = TAG_USER + 0x0500_3004;
const LISTBROWSER_MAKE_VISIBLE: u32 = TAG_USER + 0x0500_3009;
const LISTBROWSER_SHOW_SELECTED: u32 = TAG_USER + 0x0500_3012;
const LBNA_COLUMN: u32 = TAG_USER + 0x0500_3504;
const LBNCA_TEXT: u32 = TAG_USER + 0x0500_3505;
const LBNCA_COPY_TEXT: u32 = TAG_USER + 0x0500_350f;
const FUELGAUGE_MIN: u32 = TAG_USER + 0x0501_2001;
const FUELGAUGE_MAX: u32 = TAG_USER + 0x0501_2002;
const FUELGAUGE_LEVEL: u32 = TAG_USER + 0x0501_2003;
const FUELGAUGE_PERCENT: u32 = TAG_USER + 0x0501_2005;
const LABEL_TEXT: u32 = TAG_USER + 0x0500_6001;
const WA_LEFT: u32 = TAG_USER + 0x64;
const WA_TOP: u32 = TAG_USER + 0x65;
const WA_IDCMP: u32 = TAG_USER + 0x6a;
const WA_TITLE: u32 = TAG_USER + 0x6e;
const WA_INNER_WIDTH: u32 = TAG_USER + 0x76;
const WA_INNER_HEIGHT: u32 = TAG_USER + 0x77;
const WA_PUB_SCREEN: u32 = TAG_USER + 0x79;
const WA_SIZE_GADGET: u32 = TAG_USER + 0x81;
const WA_DRAG_BAR: u32 = TAG_USER + 0x82;
const WA_DEPTH_GADGET: u32 = TAG_USER + 0x83;
const WA_CLOSE_GADGET: u32 = TAG_USER + 0x84;
const WA_ACTIVATE: u32 = TAG_USER + 0x89;
const WA_NEW_LOOK_MENUS: u32 = TAG_USER + 99 + 0x30;
const IDCMP_NEWSIZE: u32 = 0x2;
const IDCMP_GADGETUP: u32 = 0x40;
const IDCMP_CLOSEWINDOW: u32 = 0x200;
const IDCMP_RAWKEY: u32 = 0x400;
const IDCMP_VANILLAKEY: u32 = 0x0020_0000;
const IDCMP_IDCMPUPDATE: u32 = 0x0080_0000;
const WINDOW_VERT_PROP: u32 = TAG_USER + 0x0502_501b;
const WINDOW_VERT_OBJECT: u32 = TAG_USER + 0x0502_501c;
const WINDOW_HORIZ_PROP: u32 = TAG_USER + 0x0502_501d;
const WINDOW_HORIZ_OBJECT: u32 = TAG_USER + 0x0502_501e;
const PGA_TOTAL: u32 = TAG_USER + 0x0003_1007;
const PGA_VISIBLE: u32 = TAG_USER + 0x0003_1008;
const PGA_TOP: u32 = TAG_USER + 0x0003_1009;
const ICA_TARGET: u32 = TAG_USER + 0x0004_0001;
const ICTARGET_IDCMP: u32 = !0;
const WMHI_IGNORE: u32 = !0;
const WMHI_CLASSMASK: u32 = 0xffff_0000;
const WMHI_CLOSEWINDOW: u32 = 1 << 16;
const WMHI_GADGETUP: u32 = 2 << 16;
const WMHI_INACTIVE: u32 = 3 << 16;
const WMHI_ACTIVE: u32 = 4 << 16;
const WMHI_NEWSIZE: u32 = 5 << 16;
const WMHI_RAWKEY: u32 = 11 << 16;
const WMHI_VANILLAKEY: u32 = 12 << 16;

/// One BOOPSI class, opened once (OpenClass) and kept.
struct ClassSlot {
    name: &'static [u8],
    class: AtomicPtr<APTR>,
}

impl ClassSlot {
    const fn new(name: &'static [u8]) -> Self {
        Self { name, class: AtomicPtr::new(core::ptr::null_mut()) }
    }

    fn get(&self, i: *mut IntuitionIFace) -> Result<*mut APTR> {
        let c = self.class.load(Ordering::Acquire);
        if !c.is_null() {
            return Ok(c);
        }
        let mut class: *mut APTR = core::ptr::null_mut();
        // SAFETY: an open interface, a NUL-terminated class name (a literal
        // below) and a pointer to write the class to.
        let lib = unsafe { ((*i).OpenClass)(i, self.name.as_ptr(), 50, &mut class) };
        if lib.is_null() || class.is_null() {
            return Err(AmigaError::Unsupported("a ReAction class could not be opened"));
        }
        if self.class.compare_exchange(core::ptr::null_mut(), class, Ordering::AcqRel, Ordering::Acquire).is_err() {
            // Another task opened it first: give this open back.
            // SAFETY: the class library opened just above, unused.
            unsafe { ((*i).CloseClass)(i, lib) };
        }
        Ok(self.class.load(Ordering::Acquire))
    }
}

static WINDOW_CLASS: ClassSlot = ClassSlot::new(b"window.class\0");
static LAYOUT_CLASS: ClassSlot = ClassSlot::new(b"gadgets/layout.gadget\0");
static BUTTON_CLASS: ClassSlot = ClassSlot::new(b"gadgets/button.gadget\0");
static STRING_CLASS: ClassSlot = ClassSlot::new(b"gadgets/string.gadget\0");
static CHECKBOX_CLASS: ClassSlot = ClassSlot::new(b"gadgets/checkbox.gadget\0");
static INTEGER_CLASS: ClassSlot = ClassSlot::new(b"gadgets/integer.gadget\0");
static SLIDER_CLASS: ClassSlot = ClassSlot::new(b"gadgets/slider.gadget\0");
static CHOOSER_CLASS: ClassSlot = ClassSlot::new(b"gadgets/chooser.gadget\0");
static LISTBROWSER_CLASS: ClassSlot = ClassSlot::new(b"gadgets/listbrowser.gadget\0");
static FUELGAUGE_CLASS: ClassSlot = ClassSlot::new(b"gadgets/fuelgauge.gadget\0");
static SPACE_CLASS: ClassSlot = ClassSlot::new(b"gadgets/space.gadget\0");
static SPEEDBAR_CLASS: ClassSlot = ClassSlot::new(b"gadgets/speedbar.gadget\0");
static BITMAP_CLASS: ClassSlot = ClassSlot::new(b"images/bitmap.image\0");
static LABEL_CLASS: ClassSlot = ClassSlot::new(b"images/label.image\0");

/// What a gadget is, with its starting values.
#[derive(Clone, Debug)]
enum Kind {
    Button { text: String, push: bool, selected: bool },
    String { text: String, max_chars: u32 },
    CheckBox { text: String, checked: bool },
    Integer { value: i32, min: i32, max: i32 },
    Slider { min: i32, max: i32, level: i32, vertical: bool },
    Chooser { items: Vec<String>, active: u32 },
    List { items: Vec<String>, selected: Option<u32> },
    Gauge { min: i32, max: i32, level: i32 },
    Space,
    Toolbar(Vec<ToolButton>),
}

/// A button of a [`Gadget::toolbar`]: an image from the first of its
/// image files that exists (24x24 PNGs, like Workbench's toolbars), else
/// its label as text; or a separator line.
#[derive(Clone, Debug)]
pub struct ToolButton {
    id: u16,
    label: String,
    images: Vec<String>,
    help: Option<String>,
    hint: Option<String>,
    toggle: Option<bool>,
    separator: bool,
}

impl ToolButton {
    /// A button reported as `Event::Gadget { id: <the toolbar's id>,
    /// code: id }`.
    pub fn new(id: u16, label: &str) -> Self {
        Self { id, label: label.into(), images: Vec::new(), help: None, hint: None, toggle: None, separator: false }
    }

    /// An image file to show instead of the label (bitmap.image); several
    /// may be given, the first that exists is used.
    pub fn image(mut self, path: &str) -> Self {
        self.images.push(path.into());
        self
    }

    /// Help text, shown in the window title while the button is under
    /// the mouse.
    pub fn help(mut self, text: &str) -> Self {
        self.help = Some(text.into());
        self
    }

    /// A help bubble shown when the mouse rests on the button
    /// (SBNA_HintInfo; the window turns gadget help on).
    pub fn hint(mut self, text: &str) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// A toggle button: it stays pressed in while on (`on` at the start).
    /// A click flips it; [`Window::set_tool_selected`] sets it, and
    /// [`Window::tool_selected`] reads it.
    pub fn toggle(mut self, on: bool) -> Self {
        self.toggle = Some(on);
        self
    }

    /// A separator line between groups of buttons.
    pub fn separator() -> Self {
        Self { id: 0, label: String::new(), images: Vec::new(), help: None, hint: None, toggle: None, separator: true }
    }
}

/// The kind of a gadget in a window, for value access by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GadgetKind {
    Button,
    String,
    CheckBox,
    Integer,
    Slider,
    Chooser,
    List,
    Gauge,
    /// An empty area for the program to draw in.
    Space,
    /// A row of buttons (speedbar.gadget).
    Toolbar,
}

impl Kind {
    fn kind(&self) -> GadgetKind {
        match self {
            Kind::Button { .. } => GadgetKind::Button,
            Kind::String { .. } => GadgetKind::String,
            Kind::CheckBox { .. } => GadgetKind::CheckBox,
            Kind::Integer { .. } => GadgetKind::Integer,
            Kind::Slider { .. } => GadgetKind::Slider,
            Kind::Chooser { .. } => GadgetKind::Chooser,
            Kind::List { .. } => GadgetKind::List,
            Kind::Gauge { .. } => GadgetKind::Gauge,
            Kind::Space => GadgetKind::Space,
            Kind::Toolbar(_) => GadgetKind::Toolbar,
        }
    }
}

/// How a child sits in its layout.
#[derive(Clone, Debug, Default)]
struct Child {
    label: Option<String>,
    weight: Option<u32>,
    min_width: Option<u32>,
    min_height: Option<u32>,
}

/// One gadget, before the window is opened. Made by the constructors
/// below; `id` is what [`Event::Gadget`] reports and what the value
/// methods of [`Window`] take.
#[derive(Clone, Debug)]
pub struct Gadget {
    id: u16,
    kind: Kind,
    disabled: bool,
    child: Child,
    context: Option<crate::menu::Strip>,
    hint: Option<String>,
}

impl Gadget {
    fn new(id: u16, kind: Kind) -> Self {
        Self { id, kind, disabled: false, child: Child::default(), context: None, hint: None }
    }

    /// A button (button.gadget) with this text.
    pub fn button(id: u16, text: &str) -> Self {
        Self::new(id, Kind::Button { text: text.into(), push: false, selected: false })
    }

    /// A text field (string.gadget); Return or leaving it sends a
    /// [`Event::Gadget`].
    pub fn string(id: u16) -> Self {
        Self::new(id, Kind::String { text: String::new(), max_chars: 256 })
    }

    /// A check box (checkbox.gadget) with this text beside it.
    pub fn checkbox(id: u16, text: &str) -> Self {
        Self::new(id, Kind::CheckBox { text: text.into(), checked: false })
    }

    /// A number field with arrows (integer.gadget), 0..=100 to start.
    pub fn integer(id: u16) -> Self {
        Self::new(id, Kind::Integer { value: 0, min: 0, max: 100 })
    }

    /// A horizontal slider (slider.gadget), 0..=100 to start.
    pub fn slider(id: u16) -> Self {
        Self::new(id, Kind::Slider { min: 0, max: 100, level: 0, vertical: false })
    }

    /// A pop-up choice (chooser.gadget) of these items.
    pub fn chooser(id: u16, items: &[&str]) -> Self {
        Self::new(id, Kind::Chooser { items: items.iter().map(|s| String::from(*s)).collect(), active: 0 })
    }

    /// A list of rows to pick from (listbrowser.gadget, one column).
    pub fn list(id: u16, items: &[&str]) -> Self {
        Self::new(id, Kind::List { items: items.iter().map(|s| String::from(*s)).collect(), selected: None })
    }

    /// A progress bar (fuelgauge.gadget), 0..=100, shown as a percentage.
    pub fn gauge(id: u16) -> Self {
        Self::new(id, Kind::Gauge { min: 0, max: 100, level: 0 })
    }

    /// An empty area (space.gadget) the program draws in: its box from
    /// [`Window::render_box`], drawn through [`Window::rastport`]; give it
    /// a [`min_size`](Self::min_size). A [`context_menu`](Self::context_menu)
    /// on it is shown by the right button over the area.
    pub fn space(id: u16) -> Self {
        Self::new(id, Kind::Space)
    }

    /// A toolbar (speedbar.gadget) of these buttons; a click is
    /// `Event::Gadget { id, code: <the button's id> }`. In a vertical
    /// layout give it `weight(0)` to keep its own height.
    pub fn toolbar(id: u16, buttons: Vec<ToolButton>) -> Self {
        Self::new(id, Kind::Toolbar(buttons))
    }

    /// The starting text of a string gadget.
    pub fn text(mut self, t: &str) -> Self {
        if let Kind::String { text, .. } = &mut self.kind {
            *text = t.into();
        }
        self
    }

    /// The most characters a string gadget takes.
    pub fn max_chars(mut self, n: u32) -> Self {
        if let Kind::String { max_chars, .. } = &mut self.kind {
            *max_chars = n.max(1);
        }
        self
    }

    /// A check box ticked, or a push button down.
    pub fn checked(mut self, on: bool) -> Self {
        match &mut self.kind {
            Kind::CheckBox { checked, .. } => *checked = on,
            Kind::Button { selected, .. } => *selected = on,
            _ => {}
        }
        self
    }

    /// A button that stays down when clicked, until clicked again.
    pub fn push(mut self) -> Self {
        if let Kind::Button { push, .. } = &mut self.kind {
            *push = true;
        }
        self
    }

    /// The range of an integer, slider or gauge.
    pub fn range(mut self, lo: i32, hi: i32) -> Self {
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        match &mut self.kind {
            Kind::Integer { min, max, .. } | Kind::Slider { min, max, .. } | Kind::Gauge { min, max, .. } => {
                *min = lo;
                *max = hi;
            }
            _ => {}
        }
        self
    }

    /// The starting value of an integer, slider or gauge.
    pub fn value(mut self, v: i32) -> Self {
        match &mut self.kind {
            Kind::Integer { value, .. } => *value = v,
            Kind::Slider { level, .. } | Kind::Gauge { level, .. } => *level = v,
            _ => {}
        }
        self
    }

    /// A vertical slider.
    pub fn vertical(mut self) -> Self {
        if let Kind::Slider { vertical, .. } = &mut self.kind {
            *vertical = true;
        }
        self
    }

    /// The starting item of a chooser, or the selected row of a list.
    pub fn select(mut self, k: u32) -> Self {
        match &mut self.kind {
            Kind::Chooser { active, .. } => *active = k,
            Kind::List { selected, .. } => *selected = Some(k),
            _ => {}
        }
        self
    }

    /// Greyed out and not usable.
    pub fn disabled(mut self, on: bool) -> Self {
        self.disabled = on;
        self
    }

    /// A context menu (a strip of one menu) shown by the right button over
    /// the gadget (GA_ContextMenu); its picks arrive as [`Event::Menu`]
    /// with this gadget's id.
    pub fn context_menu(mut self, menu: crate::menu::Strip) -> Self {
        self.context = Some(menu);
        self
    }

    /// A help bubble shown when the mouse rests on the gadget
    /// (GA_HintInfo; the window turns gadget help on).
    pub fn hint(mut self, text: &str) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// A text beside the gadget in its layout (CHILD_Label).
    pub fn label(mut self, text: &str) -> Self {
        self.child.label = Some(text.into());
        self
    }

    /// Its share of the layout's room (CHILD_Weighted*; default 100).
    pub fn weight(mut self, w: u32) -> Self {
        self.child.weight = Some(w);
        self
    }

    /// The least room it takes, in pixels.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.child.min_width = Some(width);
        self.child.min_height = Some(height);
        self
    }
}

/// A group of gadgets and layouts in a row or a column (layout.gadget).
#[derive(Clone, Debug)]
pub struct Layout {
    vertical: bool,
    label: Option<String>,
    even: bool,
    children: Vec<Node>,
    child: Child,
}

/// A layout's child.
#[derive(Clone, Debug)]
pub enum Node {
    Gadget(Gadget),
    Layout(Layout),
}

impl From<Gadget> for Node {
    fn from(g: Gadget) -> Self {
        Node::Gadget(g)
    }
}

impl From<Layout> for Node {
    fn from(l: Layout) -> Self {
        Node::Layout(l)
    }
}

impl Layout {
    /// Children one above the other.
    pub fn vertical() -> Self {
        Self { vertical: true, label: None, even: false, children: Vec::new(), child: Child::default() }
    }

    /// Children side by side.
    pub fn horizontal() -> Self {
        Self { vertical: false, ..Self::vertical() }
    }

    /// Add a gadget or a layout.
    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }

    /// A framed group with this title.
    pub fn label(mut self, text: &str) -> Self {
        self.label = Some(text.into());
        self
    }

    /// All children the same size (LAYOUT_EvenSize).
    pub fn even(mut self) -> Self {
        self.even = true;
        self
    }

    /// Its share of the parent layout's room (default 100).
    pub fn weight(mut self, w: u32) -> Self {
        self.child.weight = Some(w);
        self
    }
}

/// How to open a [`Window`].
#[derive(Clone, Copy, Debug)]
pub struct WindowSpec<'a> {
    pub title: &'a str,
    /// Place; None centres it on the screen.
    pub position: Option<(i32, i32)>,
    /// Inside size; None: the layout's own size.
    pub inner_size: Option<(u32, u32)>,
    pub activate: bool,
    pub size_gadget: bool,
    pub close_gadget: bool,
    /// The window's menus (built with menuclass, owned by the window).
    pub menu: Option<&'a crate::menu::Strip>,
    /// The screen's title while the window is active.
    pub screen_title: Option<&'a str>,
    /// With no `position`: centred on the mouse instead of the screen.
    pub centre_on_mouse: bool,
    /// Simple refresh: Intuition keeps no copy of what is hidden, and the
    /// program redraws its own drawing after [`Event::NewSize`] and
    /// [`Event::Uniconified`] (smart refresh, the default, keeps it).
    pub simple_refresh: bool,
    /// An iconify gadget: the window goes to a Workbench icon
    /// ([`Event::Iconified`]) and comes back when it is double-clicked
    /// ([`Event::Uniconified`]).
    pub iconify: bool,
    /// Files dropped on the window from Workbench arrive as
    /// [`Event::Dropped`] (the window is an AppWindow).
    pub drops: bool,
    /// The iconified icon's label (default: `title` as it was at open).
    pub icon_title: Option<&'a str>,
    /// Scrollers in the right and bottom window borders; set them with
    /// [`Window::set_scroll`], moves arrive as [`Event::Scroll`].
    pub scrollers: bool,
}

impl Default for WindowSpec<'_> {
    fn default() -> Self {
        Self {
            title: "",
            position: None,
            inner_size: None,
            activate: true,
            size_gadget: true,
            close_gadget: true,
            menu: None,
            screen_title: None,
            centre_on_mouse: false,
            simple_refresh: false,
            iconify: false,
            drops: false,
            icon_title: None,
            scrollers: false,
        }
    }
}

/// What happened in a [`Window`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The close gadget.
    Close,
    /// A gadget was released: a button clicked, a string ended with
    /// Return, a check box, slider, chooser or list changed. `code` is
    /// the gadget's code (a slider's level, a chooser's item, a list's row).
    Gadget { id: u16, code: i16 },
    /// A key with no gadget active (VANILLAKEY).
    Key(u8),
    /// A menu item picked: its id, whether it is now checked (toggles and
    /// radio items), and the gadget whose context menu it was in (None:
    /// the window's menus).
    Menu { id: u32, checked: bool, gadget: Option<u16> },
    /// A raw key (cursor keys, function keys) with the qualifiers held
    /// (WINDOW_Qualifier: shift 0x0003, alt 0x0030, ...).
    RawKey { code: u16, qualifier: u16 },
    NewSize,
    Active,
    Inactive,
    /// The window was iconified (it is closed; its icon is on Workbench).
    Iconified,
    /// The window is open again: redraw what the program draws.
    Uniconified,
    /// Files were dropped on the window: [`Window::dropped`] gives them.
    Dropped,
    /// A border scroller ([`WindowSpec::scrollers`]) was moved: its new
    /// top (in the units of [`Window::set_scroll`]).
    Scroll { vertical: bool, top: u32 },
    /// Anything else window.class reported (the WMHI class and data).
    Other(u32),
}

/// The objects and memory one window keeps.
struct Built {
    gadgets: Vec<(u16, GadgetKind, *mut APTR)>,
    strings: Vec<Vec<u8>>,
    arrays: Vec<Vec<*const u8>>,
    lists: Vec<(u16, Box<[u32; 4]>)>,
    /// The window's menu tree (None) and the gadgets' context menus,
    /// disposed after the window object.
    menus: Vec<(Option<u16>, crate::menu::MenuTree)>,
    /// Toolbars' button lists and button images, freed after the window
    /// object (the speedbar frees neither).
    toolbars: Vec<Toolbar>,
    images: Vec<*mut APTR>,
    /// The screen the images are made for.
    screen: *mut sys::Screen,
    /// Some gadget has a hint: the window turns gadget help on.
    hints: bool,
    /// Minimum sizes measured for gadgets (toolbars), by object.
    min_sizes: Vec<(*mut APTR, u32, u32)>,
}

impl Built {
    fn text(&mut self, s: &str) -> Result<u32> {
        let c = c_copy(s.as_bytes())?;
        // The Vec's heap block does not move when `strings` grows.
        let p = c.as_ptr() as u32;
        self.strings.push(c);
        Ok(p)
    }

    /// A speedbar's node list for `buttons`: each an image (bitmap.image
    /// from the first file that exists) or its label as text.
    fn toolbar(&mut self, i: *mut IntuitionIFace, id: u16, buttons: &[ToolButton]) -> Result<*mut sys::List> {
        let sb = sb_iface()?;
        let mut list = Box::new([0u32; 4]);
        let lp = list.as_mut_ptr() as *mut sys::List;
        // SAFETY: a zeroed List-sized block, initialised by NewList.
        unsafe { sys::exec_new_list(lp) };
        // Kept (and freed) even if a button fails below.
        self.toolbars.push(Toolbar { id, list, nodes: Vec::new() });
        let k = self.toolbars.len() - 1;
        for bt in buttons {
            let mut t: Vec<TagItem> = Vec::new();
            if bt.separator {
                t.push(TagItem { ti_Tag: SBNA_SEPARATOR, ti_Data: 1 });
            } else {
                t.push(TagItem { ti_Tag: SBNA_BUTTON_ID, ti_Data: bt.id as u32 });
                let image = bt.images.iter().find(|p| crate::dos::fs::Lock::new(p, crate::dos::fs::LockMode::Shared).is_ok()).and_then(|p| self.image(i, p).ok());
                match image {
                    Some(img) => t.push(TagItem { ti_Tag: SBNA_IMAGE, ti_Data: img as u32 }),
                    None => t.push(TagItem { ti_Tag: SBNA_TEXT, ti_Data: self.text(&bt.label)? }),
                }
                if let Some(h) = &bt.help {
                    t.push(TagItem { ti_Tag: SBNA_HELP, ti_Data: self.text(h)? });
                }
                if let Some(h) = &bt.hint {
                    t.push(TagItem { ti_Tag: SBNA_HINT_INFO, ti_Data: self.text(h)? });
                    self.hints = true;
                }
                if let Some(on) = bt.toggle {
                    t.push(TagItem { ti_Tag: SBNA_TOGGLE, ti_Data: 1 });
                    t.push(TagItem { ti_Tag: SBNA_SELECTED, ti_Data: on as u32 });
                }
            }
            t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
            // SAFETY: an open interface, the button's number and a
            // TAG_DONE-ended list whose texts and image live in `self`.
            let node = unsafe { ((*sb).AllocSpeedButtonNodeA)(sb, bt.id, t.as_mut_ptr()) };
            if node.is_null() {
                return Err(AmigaError::AllocationFailed);
            }
            // SAFETY: a new node, added to our list once.
            unsafe { sys::exec_add_tail(lp, node) };
            if !bt.separator {
                self.toolbars[k].nodes.push((bt.id, node));
            }
        }
        Ok(lp)
    }

    /// A bitmap.image of `path` for the window's screen; disposed with
    /// the window.
    fn image(&mut self, i: *mut IntuitionIFace, path: &str) -> Result<*mut APTR> {
        let tags = [
            TagItem { ti_Tag: BITMAP_SOURCE_FILE, ti_Data: self.text(path)? },
            TagItem { ti_Tag: BITMAP_SCREEN, ti_Data: self.screen as u32 },
            TagItem { ti_Tag: BITMAP_MASKING, ti_Data: 1 },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        let o = new_object(i, BITMAP_CLASS.get(i)?, &tags)?;
        self.images.push(o);
        Ok(o)
    }

    /// A list browser node list of `items` (one column, texts copied).
    fn list(&mut self, id: u16, items: &[String]) -> Result<*mut sys::List> {
        let lb = lb_iface()?;
        // An exec List: 14 bytes, longword aligned.
        let mut list = Box::new([0u32; 4]);
        let lp = list.as_mut_ptr() as *mut sys::List;
        // SAFETY: a zeroed List-sized block, initialised by NewList.
        unsafe { sys::exec_new_list(lp) };
        for it in items {
            let t = c_copy(it.as_bytes())?;
            let tags = [
                TagItem { ti_Tag: LBNA_COLUMN, ti_Data: 0 },
                TagItem { ti_Tag: LBNCA_COPY_TEXT, ti_Data: 1 },
                TagItem { ti_Tag: LBNCA_TEXT, ti_Data: t.as_ptr() as u32 },
                TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
            ];
            // SAFETY: an open interface, one column, a TAG_DONE-ended list
            // whose text is copied into the node (LBNCA_CopyText).
            let node = unsafe { ((*lb).AllocListBrowserNodeA)(lb, 1, tags.as_ptr() as *mut TagItem) };
            if node.is_null() {
                // SAFETY: our list with the nodes added so far.
                unsafe { ((*lb).FreeListBrowserList)(lb, lp) };
                return Err(AmigaError::AllocationFailed);
            }
            // SAFETY: a new node, added to our list once.
            unsafe { sys::exec_add_tail(lp, node) };
        }
        self.lists.push((id, list));
        Ok(lp)
    }
}

fn new_object(i: *mut IntuitionIFace, class: *mut APTR, tags: &[TagItem]) -> Result<*mut APTR> {
    // SAFETY: an open interface, an open class and a TAG_DONE-ended list
    // whose pointers live in `Built` (or the caller) past the object.
    let o = unsafe { ((*i).NewObjectA)(i, class, core::ptr::null_mut(), tags.as_ptr()) };
    if o.is_null() { Err(AmigaError::AllocationFailed) } else { Ok(o) }
}

fn dispose(i: *mut IntuitionIFace, o: *mut APTR) {
    // SAFETY: an object made by new_object, owned by no layout, disposed
    // once.
    unsafe { ((*i).DisposeObject)(i, o) };
}

fn gadget_object(i: *mut IntuitionIFace, g: &Gadget, b: &mut Built) -> Result<*mut APTR> {
    let mut t: Vec<TagItem> = Vec::new();
    let mut tag = |a: u32, d: u32| t.push(TagItem { ti_Tag: a, ti_Data: d });
    tag(GA_ID, g.id as u32);
    tag(GA_RELVERIFY, 1);
    tag(GA_DISABLED, g.disabled as u32);
    let class = match &g.kind {
        Kind::Button { text, push, selected } => {
            tag(GA_TEXT, b.text(text)?);
            if *push {
                tag(BUTTON_PUSH_BUTTON, 1);
                tag(GA_SELECTED, *selected as u32);
            }
            &BUTTON_CLASS
        }
        Kind::String { text, max_chars } => {
            tag(GA_TAB_CYCLE, 1);
            tag(STRINGA_MAX_CHARS, *max_chars);
            tag(STRINGA_TEXT_VAL, b.text(text)?);
            &STRING_CLASS
        }
        Kind::CheckBox { text, checked } => {
            tag(GA_TEXT, b.text(text)?);
            tag(CHECKBOX_CHECKED, *checked as u32);
            &CHECKBOX_CLASS
        }
        Kind::Integer { value, min, max } => {
            tag(GA_TAB_CYCLE, 1);
            tag(INTEGER_MINIMUM, *min as u32);
            tag(INTEGER_MAXIMUM, *max as u32);
            tag(INTEGER_NUMBER, *value as u32);
            tag(INTEGER_ARROWS, 1);
            &INTEGER_CLASS
        }
        Kind::Slider { min, max, level, vertical } => {
            tag(SLIDER_MIN, *min as u32);
            tag(SLIDER_MAX, *max as u32);
            tag(SLIDER_LEVEL, *level as u32);
            tag(SLIDER_ORIENTATION, if *vertical { SORIENT_VERT } else { SORIENT_HORIZ });
            &SLIDER_CLASS
        }
        Kind::Chooser { items, active } => {
            let mut arr: Vec<*const u8> = Vec::with_capacity(items.len() + 1);
            for it in items {
                arr.push(b.text(it)? as *const u8);
            }
            arr.push(core::ptr::null());
            tag(CHOOSER_POP_UP, 1);
            tag(CHOOSER_LABEL_ARRAY, arr.as_ptr() as u32);
            tag(CHOOSER_ACTIVE, *active);
            b.arrays.push(arr);
            &CHOOSER_CLASS
        }
        Kind::List { items, selected } => {
            let lp = b.list(g.id, items)?;
            tag(LISTBROWSER_LABELS, lp as u32);
            tag(LISTBROWSER_SHOW_SELECTED, 1);
            tag(LISTBROWSER_SELECTED, selected.map(|s| s as i32).unwrap_or(-1) as u32);
            &LISTBROWSER_CLASS
        }
        Kind::Gauge { min, max, level } => {
            tag(FUELGAUGE_MIN, *min as u32);
            tag(FUELGAUGE_MAX, *max as u32);
            tag(FUELGAUGE_LEVEL, *level as u32);
            tag(FUELGAUGE_PERCENT, 1);
            &FUELGAUGE_CLASS
        }
        Kind::Space => {
            // The program draws the inside; no background is cleared.
            tag(SPACE_TRANSPARENT, 1);
            &SPACE_CLASS
        }
        Kind::Toolbar(buttons) => {
            let lp = b.toolbar(i, g.id, buttons)?;
            tag(SPEEDBAR_BUTTONS, lp as u32);
            &SPEEDBAR_CLASS
        }
    };
    // A toolbar with button hints needs a hint of its own for bubbles to
    // be looked for at all (SBNA_HintInfo replaces it over a button).
    // Empty: over the bar's empty part there is nothing to say (a button's
    // text there was wrong, seen on an X5000).
    let hint = g.hint.clone().or_else(|| match &g.kind {
        Kind::Toolbar(bs) if bs.iter().any(|b| b.hint.is_some()) => Some(String::new()),
        _ => None,
    });
    if let Some(h) = hint {
        tag(GA_HINT_INFO, b.text(&h)?);
        b.hints = true;
    }
    if let Some(strip) = &g.context {
        let tree = crate::menu::MenuTree::new(strip)?;
        tag(GA_CONTEXT_MENU, tree.as_ptr() as u32);
        b.menus.push((Some(g.id), tree));
    }
    tag(TAG_DONE, 0);
    let o = new_object(i, class.get(i)?, &t)?;
    b.gadgets.push((g.id, g.kind.kind(), o));
    if let Kind::Toolbar(_) = g.kind {
        // A speedbar's minimum is a squashed, scrolling sliver: keep all
        // its buttons in view at full size (its nominal width and height)
        // unless the program set a minimum.
        if !b.screen.is_null() {
            let mut d = GpDomain {
                method: GM_DOMAIN,
                ginfo: core::ptr::null_mut(),
                // SAFETY: the locked screen's own RastPort (an embedded
                // struct at a checked offset), measured for, not drawn in.
                rport: unsafe { (b.screen as *mut u8).add(SCREEN_RASTPORT) } as *mut sys::RastPort,
                which: GDOMAIN_NOMINAL,
                domain: [0; 4],
                attrs: core::ptr::null_mut(),
            };
            // SAFETY: our new speedbar and a filled gpDomain it writes into.
            unsafe { ((*i).IDoMethodA)(i, o, &mut d as *mut GpDomain as APTR) };
            if d.domain[2] > 0 && d.domain[3] > 0 {
                b.min_sizes.push((o, d.domain[2] as u32, d.domain[3] as u32));
            }
        }
    }
    Ok(o)
}

/// The child tags after a LAYOUT_AddChild; a label image is made here.
fn child_tags(i: *mut IntuitionIFace, o: *mut APTR, c: &Child, vertical: bool, b: &mut Built, t: &mut Vec<TagItem>, images: &mut Vec<*mut APTR>) -> Result<()> {
    if let Some(l) = &c.label {
        let tags = [TagItem { ti_Tag: LABEL_TEXT, ti_Data: b.text(l)? }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        let img = new_object(i, LABEL_CLASS.get(i)?, &tags)?;
        images.push(img);
        t.push(TagItem { ti_Tag: CHILD_LABEL, ti_Data: img as u32 });
    }
    if let Some(w) = c.weight {
        t.push(TagItem { ti_Tag: if vertical { CHILD_WEIGHTED_HEIGHT } else { CHILD_WEIGHTED_WIDTH }, ti_Data: w });
    }
    let measured = b.min_sizes.iter().find(|m| m.0 == o);
    if let Some(w) = c.min_width.or(measured.map(|m| m.1)) {
        t.push(TagItem { ti_Tag: CHILD_MIN_WIDTH, ti_Data: w });
    }
    if let Some(h) = c.min_height.or(measured.map(|m| m.2)) {
        t.push(TagItem { ti_Tag: CHILD_MIN_HEIGHT, ti_Data: h });
    }
    Ok(())
}

/// Make a layout and its children. On failure every object made here is
/// disposed (the gadgets recorded in `b` are dropped from it too).
fn layout_object(i: *mut IntuitionIFace, l: &Layout, b: &mut Built) -> Result<*mut APTR> {
    let mut t: Vec<TagItem> = Vec::new();
    t.push(TagItem { ti_Tag: LAYOUT_ORIENTATION, ti_Data: if l.vertical { LAYOUT_ORIENT_VERT } else { LAYOUT_ORIENT_HORIZ } });
    t.push(TagItem { ti_Tag: LAYOUT_SPACE_OUTER, ti_Data: 1 });
    t.push(TagItem { ti_Tag: LAYOUT_SPACE_INNER, ti_Data: 1 });
    if l.even {
        t.push(TagItem { ti_Tag: LAYOUT_EVEN_SIZE, ti_Data: 1 });
    }
    if let Some(lab) = &l.label {
        t.push(TagItem { ti_Tag: LAYOUT_BEVEL_STYLE, ti_Data: BVS_GROUP });
        t.push(TagItem { ti_Tag: LAYOUT_LABEL, ti_Data: b.text(lab)? });
    }
    let first_gadget = b.gadgets.len();
    let mut made: Vec<*mut APTR> = Vec::new();
    let mut images: Vec<*mut APTR> = Vec::new();
    let mut result = Ok(());
    for n in &l.children {
        let (obj, child) = match n {
            Node::Gadget(g) => (gadget_object(i, g, b), &g.child),
            Node::Layout(sub) => (layout_object(i, sub, b), &sub.child),
        };
        match obj {
            Ok(o) => {
                made.push(o);
                t.push(TagItem { ti_Tag: LAYOUT_ADD_CHILD, ti_Data: o as u32 });
                if let Err(e) = child_tags(i, o, child, l.vertical, b, &mut t, &mut images) {
                    result = Err(e);
                    break;
                }
            }
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }
    t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
    let obj = result.and_then(|_| new_object(i, LAYOUT_CLASS.get(i)?, &t));
    if obj.is_err() {
        // Nothing owns the children: dispose them (a sub-layout takes its
        // own children with it) and forget their gadgets.
        for o in made.into_iter().chain(images) {
            dispose(i, o);
        }
        b.gadgets.truncate(first_gadget);
    }
    obj
}

/// Dispose of a registered window object (the panic path; see
/// `crate::track`): it closes the window and disposes the gadgets. The
/// lists and texts the gadgets showed are leaked.
unsafe fn release_ra_window(i: usize, obj: usize) {
    dispose(i as *mut IntuitionIFace, obj as *mut APTR);
}

/// An open ReAction window; the window object, its layout and gadgets are
/// disposed on drop.
pub struct Window<'s> {
    i: *mut IntuitionIFace,
    obj: *mut APTR,
    // The Intuition window; NULL while iconified.
    win: core::cell::Cell<*mut sys::Window>,
    built: Built,
    // The AppPort for iconifying and drops, and the hook that queues the
    // dropped files (it runs inside WM_HANDLEINPUT, in this task).
    port: Option<crate::port::AmigaMsgPort>,
    _drop_hook: Option<Box<dyn core::any::Any>>,
    drops: Box<core::cell::RefCell<alloc::collections::VecDeque<Vec<String>>>>,
    // The title, kept while the window object points at it.
    title: core::cell::RefCell<Vec<u8>>,
    // Further items of a multiple menu selection, delivered next.
    queue: core::cell::RefCell<alloc::collections::VecDeque<Event>>,
    // The border scrollers' (total, visible, top) as last set or seen:
    // [horizontal, vertical]; None without scrollers.
    scroll: Option<core::cell::Cell<[(u32, u32, u32); 2]>>,
    _t: crate::track::Tracked,
    _screen: PhantomData<&'s PubScreen>,
}

impl<'s> Window<'s> {
    /// Make the objects for `root` and open the window on `screen`.
    #[track_caller]
    pub fn open(screen: &'s PubScreen, spec: &WindowSpec<'_>, root: Layout) -> Result<Self> {
        let i = crate::intuition::intuition_iface()?;
        let mut b = Built { gadgets: Vec::new(), strings: Vec::new(), arrays: Vec::new(), lists: Vec::new(), menus: Vec::new(), toolbars: Vec::new(), images: Vec::new(), screen: screen.as_ptr(), hints: false, min_sizes: Vec::new() };
        if let Some(strip) = spec.menu {
            b.menus.push((None, crate::menu::MenuTree::new(strip)?));
        }
        let wclass = WINDOW_CLASS.get(i)?;
        let title_text = c_copy(spec.title.as_bytes())?;
        let title = title_text.as_ptr() as u32;
        let port = if spec.iconify || spec.drops { Some(crate::port::AmigaMsgPort::new()?) } else { None };
        let drops: Box<core::cell::RefCell<alloc::collections::VecDeque<Vec<String>>>> = Box::new(core::cell::RefCell::new(alloc::collections::VecDeque::new()));
        let drop_hook = if spec.drops {
            let q = &*drops as *const core::cell::RefCell<alloc::collections::VecDeque<Vec<String>>> as usize;
            Some(crate::hook::Hook::new(move |_win, msg| {
                if !msg.is_null() {
                    let files = crate::workbench::app_message_files(msg as *const u8);
                    // SAFETY: the queue is boxed in the Window, which outlives
                    // its window object (disposed first); the hook runs only
                    // inside WM_HANDLEINPUT, which the Window calls from its
                    // own methods while no borrow of the queue is held.
                    unsafe { (*(q as *const core::cell::RefCell<alloc::collections::VecDeque<Vec<String>>>)).borrow_mut().push_back(files) };
                }
                0
            })?)
        } else {
            None
        };
        let layout = match layout_object(i, &root, &mut b) {
            Ok(l) => l,
            Err(e) => {
                free_lists(&mut b);
                return Err(e);
            }
        };
        let mut t: Vec<TagItem> = vec![
            TagItem { ti_Tag: WA_TITLE, ti_Data: title },
            TagItem { ti_Tag: WA_PUB_SCREEN, ti_Data: screen.as_ptr() as u32 },
            TagItem { ti_Tag: WA_ACTIVATE, ti_Data: spec.activate as u32 },
            TagItem { ti_Tag: WA_DRAG_BAR, ti_Data: 1 },
            TagItem { ti_Tag: WA_NEW_LOOK_MENUS, ti_Data: 1 },
            TagItem { ti_Tag: WA_DEPTH_GADGET, ti_Data: 1 },
            TagItem { ti_Tag: WA_CLOSE_GADGET, ti_Data: spec.close_gadget as u32 },
            TagItem { ti_Tag: WA_SIZE_GADGET, ti_Data: spec.size_gadget as u32 },
            TagItem { ti_Tag: WA_IDCMP, ti_Data: IDCMP_GADGETUP | IDCMP_CLOSEWINDOW | IDCMP_VANILLAKEY | IDCMP_RAWKEY | IDCMP_NEWSIZE | IDCMP_MENUPICK | if spec.scrollers { IDCMP_IDCMPUPDATE } else { 0 } },
        ];
        if spec.scrollers {
            t.push(TagItem { ti_Tag: WINDOW_VERT_PROP, ti_Data: 1 });
            t.push(TagItem { ti_Tag: WINDOW_HORIZ_PROP, ti_Data: 1 });
        }
        if let Some((_, tree)) = b.menus.iter().find(|m| m.0.is_none()) {
            t.push(TagItem { ti_Tag: WINDOW_MENU_STRIP, ti_Data: tree.as_ptr() as u32 });
        }
        match spec.position {
            Some((x, y)) => {
                t.push(TagItem { ti_Tag: WA_LEFT, ti_Data: x as u32 });
                t.push(TagItem { ti_Tag: WA_TOP, ti_Data: y as u32 });
            }
            None => t.push(TagItem { ti_Tag: WINDOW_POSITION, ti_Data: if spec.centre_on_mouse { WPOS_CENTERMOUSE } else { WPOS_CENTERSCREEN } }),
        }
        if let Some((w, h)) = spec.inner_size {
            t.push(TagItem { ti_Tag: WA_INNER_WIDTH, ti_Data: w });
            t.push(TagItem { ti_Tag: WA_INNER_HEIGHT, ti_Data: h });
        }
        if b.hints {
            t.push(TagItem { ti_Tag: WINDOW_GADGET_HELP, ti_Data: 1 });
        }
        if let Some(st) = spec.screen_title {
            t.push(TagItem { ti_Tag: WA_SCREEN_TITLE, ti_Data: b.text(st)? });
        }
        t.push(TagItem { ti_Tag: if spec.simple_refresh { WA_SIMPLE_REFRESH } else { WA_SMART_REFRESH }, ti_Data: 1 });
        if let Some(p) = &port {
            t.push(TagItem { ti_Tag: WINDOW_APP_PORT, ti_Data: p.as_ptr() as u32 });
            if spec.iconify {
                t.push(TagItem { ti_Tag: WINDOW_ICONIFY_GADGET, ti_Data: 1 });
                // The icon's label: its own copy, kept for the window's life
                // (the title string is replaced by set_title, and window.class
                // keeps the pointer it was given).
                t.push(TagItem { ti_Tag: WINDOW_ICON_TITLE, ti_Data: b.text(spec.icon_title.unwrap_or(spec.title))? });
            }
            if let Some(h) = &drop_hook {
                t.push(TagItem { ti_Tag: WINDOW_APP_WINDOW, ti_Data: 1 });
                t.push(TagItem { ti_Tag: WINDOW_APP_MSG_HOOK, ti_Data: h.as_ptr() as u32 });
            }
        }
        t.push(TagItem { ti_Tag: WINDOW_LAYOUT, ti_Data: layout as u32 });
        t.push(TagItem { ti_Tag: TAG_DONE, ti_Data: 0 });
        let obj = match new_object(i, wclass, &t) {
            Ok(o) => o,
            Err(e) => {
                dispose(i, layout);
                free_lists(&mut b);
                return Err(e);
            }
        };
        let msg = [WM_OPEN, 0u32];
        // SAFETY: our window object and a WM_OPEN message (MethodID, no
        // GadgetInfo); it answers the opened struct Window or NULL.
        let win = unsafe { ((*i).IDoMethodA)(i, obj, msg.as_ptr() as APTR) } as *mut sys::Window;
        let w = Self {
            i,
            obj,
            win: core::cell::Cell::new(win),
            built: b,
            port,
            _drop_hook: drop_hook.map(|h| Box::new(h) as Box<dyn core::any::Any>),
            drops,
            title: core::cell::RefCell::new(title_text),
            queue: core::cell::RefCell::new(alloc::collections::VecDeque::new()),
            scroll: spec.scrollers.then(|| core::cell::Cell::new([(0, 0, 0); 2])),
            _t: crate::track::register("reaction window", i as usize, obj as usize, release_ra_window),
            _screen: PhantomData,
        };
        if win.is_null() {
            // Drop disposes the object (and the layout it owns).
            drop(w);
            return Err(AmigaError::Unsupported("the window could not be opened"));
        }
        // The scrollers report each move to the window's port (an
        // IDCMPUPDATE), so a drag wakes the program; next() reads where
        // they are.
        for v in [false, true] {
            if let Some(o) = w.scroller(v) {
                let tags = [TagItem { ti_Tag: ICA_TARGET, ti_Data: ICTARGET_IDCMP }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
                // SAFETY: the window's own scroller object and a TAG_DONE-ended list.
                unsafe { ((*i).SetAttrsA)(i, o, tags.as_ptr() as *mut TagItem) };
            }
        }
        Ok(w)
    }

    /// The signals to Wait on for this window's events (its IDCMP port's,
    /// and its AppPort's for drops and the iconified icon).
    pub fn signal_mask(&self) -> u32 {
        self.attr(self.obj, WINDOW_SIG_MASK).unwrap_or(0) | self.port.as_ref().map(|p| p.signal_mask()).unwrap_or(0)
    }

    /// The Intuition window (for drawing or the crate's other calls);
    /// NULL while iconified.
    pub fn as_ptr(&self) -> *mut sys::Window {
        self.win.get()
    }

    /// Whether the window is iconified (no Intuition window).
    pub fn is_iconified(&self) -> bool {
        self.win.get().is_null()
    }

    /// Iconify the window (WM_ICONIFY; needs [`WindowSpec::iconify`]).
    pub fn iconify(&self) -> bool {
        if self.port.is_none() || self.win.get().is_null() {
            return false;
        }
        let msg = [WM_ICONIFY, 0u32];
        // SAFETY: our window object with an AppPort and a WM_ICONIFY
        // message; it closes the Intuition window and puts an icon up.
        unsafe { ((*self.i).IDoMethodA)(self.i, self.obj, msg.as_ptr() as APTR) };
        self.win.set(core::ptr::null_mut());
        true
    }

    /// Open the window again after iconifying (WM_OPEN).
    pub fn uniconify(&self) -> bool {
        if !self.win.get().is_null() {
            return true;
        }
        let msg = [WM_OPEN, 0u32];
        // SAFETY: our (iconified) window object and a WM_OPEN message.
        let w = unsafe { ((*self.i).IDoMethodA)(self.i, self.obj, msg.as_ptr() as APTR) } as *mut sys::Window;
        self.win.set(w);
        !w.is_null()
    }

    /// The files of the oldest drop not yet taken (after [`Event::Dropped`]).
    pub fn dropped(&self) -> Vec<String> {
        self.drops.borrow_mut().pop_front().unwrap_or_default()
    }

    fn rd<T: Copy>(&self, off: usize) -> Option<T> {
        let w = self.win.get();
        // SAFETY: the open Intuition window and a field offset from
        // intuition.h (tests/layout/intuition.c).
        (!w.is_null()).then(|| unsafe { core::ptr::read_unaligned((w as *const u8).add(off) as *const T) })
    }

    /// Position and outer size (left, top, width, height); None while
    /// iconified.
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        Some((self.rd::<i16>(WIN_LEFT)? as i32, self.rd::<i16>(WIN_LEFT + 2)? as i32, self.rd::<i16>(WIN_LEFT + 4)? as i32, self.rd::<i16>(WIN_LEFT + 6)? as i32))
    }

    /// Draw the window's frame and gadgets again (the whole layout). After the
    /// window shrinks nothing is uncovered, and gadgets that the new
    /// layout moved can be left undrawn (imgview's toolbar, X5000): call
    /// this once a new layout is seen, before drawing over it.
    pub fn redraw_gadgets(&self) {
        let w = self.win.get();
        let Some(l) = self.attr(self.obj, WINDOW_LAYOUT) else { return };
        if !w.is_null() && l != 0 {
            // SAFETY: our open window and its own root layout gadget, drawn
            // with its children (one gadget: the layout).
            unsafe {
                ((*self.i).RefreshWindowFrame)(self.i, w);
                ((*self.i).RefreshGList)(self.i, l as *mut sys::Gadget, w, core::ptr::null_mut(), 1);
            }
        }
    }

    /// The least outer size the window can take (its MinWidth and
    /// MinHeight, which window.class sets from the layout); None while
    /// iconified.
    pub fn min_size(&self) -> Option<(i32, i32)> {
        Some((self.rd::<u16>(WIN_MIN_SIZE)? as i32, self.rd::<u16>(WIN_MIN_SIZE + 2)? as i32))
    }

    /// ChangeWindowBox (carried out later; a NewSize follows), never below
    /// the window's least size: Intuition does not hold ChangeWindowBox to
    /// the limits, and a window smaller than its layout's minimum has its
    /// gadgets overlap (a saved size from an older layout, X5000).
    pub fn change_box(&self, left: i32, top: i32, width: i32, height: i32) {
        let (mw, mh) = self.min_size().unwrap_or((0, 0));
        let (width, height) = (width.max(mw), height.max(mh));
        let w = self.win.get();
        if !w.is_null() {
            // SAFETY: our open window; Intuition limits the box to the screen.
            unsafe { ((*self.i).ChangeWindowBox)(self.i, w, left as i16, top as i16, width as i16, height as i16) };
        }
    }

    /// ActivateWindow and WindowToFront (when open).
    pub fn to_front(&self) {
        let w = self.win.get();
        if !w.is_null() {
            // SAFETY: our open window.
            unsafe {
                ((*self.i).WindowToFront)(self.i, w);
                ((*self.i).ActivateWindow)(self.i, w);
            }
        }
    }

    /// The window's title (also the icon's while iconified).
    pub fn set_title(&self, title: &str) -> Result<()> {
        let t = c_copy(title.as_bytes())?;
        let tags = [TagItem { ti_Tag: WA_TITLE, ti_Data: t.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our window object and a title kept in self (the old one
        // freed only after the object has the new one).
        unsafe { ((*self.i).SetAttrsA)(self.i, self.obj, tags.as_ptr()) };
        *self.title.borrow_mut() = t;
        Ok(())
    }

    /// The busy pointer (after a short delay), or the normal one again.
    pub fn set_busy(&self, busy: bool) {
        let w = self.win.get();
        if w.is_null() {
            return;
        }
        let tags = [TagItem { ti_Tag: WA_BUSY_POINTER, ti_Data: busy as u32 }, TagItem { ti_Tag: WA_POINTER_DELAY, ti_Data: busy as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: our open window and a TAG_DONE-ended list.
        unsafe { ((*self.i).SetWindowPointerA)(self.i, w, tags.as_ptr()) };
    }

    /// The area of a space gadget the program may draw in, in window
    /// coordinates (x0, y0, x1, y1, inclusive); None while iconified or
    /// for another kind of gadget.
    pub fn render_box(&self, id: u16) -> Option<(i32, i32, i32, i32)> {
        if self.win.get().is_null() {
            return None;
        }
        let o = self.gadget(id, &[GadgetKind::Space]).ok()?;
        let mut b = [0i16; 4];
        // SAFETY: a space gadget of this window and a struct IBox (four
        // WORDs) for SPACE_RenderBox to fill.
        let ok = unsafe { ((*self.i).GetAttr)(self.i, SPACE_RENDER_BOX, o, b.as_mut_ptr() as *mut u32) } != 0;
        if !(ok && b[2] > 0 && b[3] > 0) {
            return None;
        }
        // Never past the window's inside as it is now: until window.class
        // lays out a resized window the box is the old one, and a window
        // that has already shrunk would be drawn over its frame (imgview
        // zooming out, X5000).
        let (_, _, w, h) = self.bounds()?;
        let (bl, bt, br, bb) = (
            self.rd::<i8>(WIN_BORDERS)? as i32,
            self.rd::<i8>(WIN_BORDERS + 1)? as i32,
            self.rd::<i8>(WIN_BORDERS + 2)? as i32,
            self.rd::<i8>(WIN_BORDERS + 3)? as i32,
        );
        let (x0, y0) = ((b[0] as i32).max(bl), (b[1] as i32).max(bt));
        let (x1, y1) = ((b[0] as i32 + b[2] as i32 - 1).min(w - br - 1), (b[1] as i32 + b[3] as i32 - 1).min(h - bb - 1));
        (x1 >= x0 && y1 >= y0).then_some((x0, y0, x1, y1))
    }

    /// The window's RastPort, borrowed while `self` is; drawing is limited
    /// to the window's outer size. Err while iconified.
    #[cfg(feature = "graphics")]
    pub fn rastport(&mut self) -> Result<crate::graphics::RastPort<'_>> {
        let (_, _, w, h) = self.bounds().ok_or(AmigaError::Unsupported("the window is iconified"))?;
        let rp: *mut sys::RastPort = self.rd(WIN_RPORT).ok_or(AmigaError::NullPointer)?;
        // SAFETY: our open window's RastPort, valid while the window is
        // (the result borrows self mutably, so it cannot be iconified
        // meanwhile).
        unsafe { crate::graphics::RastPort::borrow_raw(rp, w, h) }
    }

    /// The next event, or None when there are no more for now.
    pub fn next(&self) -> Option<Event> {
        if let Some(e) = self.queue.borrow_mut().pop_front() {
            return Some(e);
        }
        let pending = self.drops.borrow().len();
        #[repr(C)]
        struct Handle {
            method: u32,
            code: *mut i16,
        }
        loop {
            let mut code: i16 = 0;
            let msg = Handle { method: WM_HANDLEINPUT, code: &mut code };
            // SAFETY: our open window object and a WM_HANDLEINPUT message
            // with a code to write, both alive for the call.
            let r = unsafe { ((*self.i).IDoMethodA)(self.i, self.obj, &msg as *const Handle as APTR) };
            if r == 0 {
                // Drops the AppMessage hook queued during WM_HANDLEINPUT.
                if self.drops.borrow().len() > pending {
                    return Some(Event::Dropped);
                }
                return self.scroll_moved();
            }
            if r == WMHI_IGNORE {
                continue;
            }
            let data = r & !WMHI_CLASSMASK;
            return Some(match r & WMHI_CLASSMASK {
                WMHI_CLOSEWINDOW => Event::Close,
                WMHI_GADGETUP => Event::Gadget { id: data as u16, code },
                WMHI_VANILLAKEY => Event::Key((data & 0xff) as u8),
                WMHI_RAWKEY => Event::RawKey { code: data as u16, qualifier: self.attr(self.obj, WINDOW_QUALIFIER).unwrap_or(0) as u16 },
                WMHI_MENUPICK => match self.menu_pick() {
                    Some(e) => e,
                    None => continue,
                },
                WMHI_ICONIFY => {
                    if !self.iconify() {
                        continue;
                    }
                    Event::Iconified
                }
                WMHI_UNICONIFY => {
                    if !self.uniconify() {
                        continue;
                    }
                    Event::Uniconified
                }
                WMHI_NEWSIZE => Event::NewSize,
                WMHI_ACTIVE => Event::Active,
                WMHI_INACTIVE => Event::Inactive,
                _ => Event::Other(r),
            });
        }
    }

    /// The picks of a WMHI_MENUPICK: the first as the event, the rest
    /// queued. The menu is the window's (WINDOW_MenuType 0) or the context
    /// menu WINDOW_MenuAddress names.
    fn menu_pick(&self) -> Option<Event> {
        let context = self.attr(self.obj, WINDOW_MENU_TYPE).unwrap_or(0) != 0;
        let (root, gadget) = if context {
            let addr = self.attr(self.obj, WINDOW_MENU_ADDRESS).unwrap_or(0) as *mut APTR;
            let g = self.built.menus.iter().find(|m| m.1.is(addr)).and_then(|m| m.0);
            (addr, g)
        } else {
            (self.built.menus.iter().find(|m| m.0.is_none()).map(|m| m.1.as_ptr()).unwrap_or(core::ptr::null_mut()), None)
        };
        let mut evs = crate::menu::selected_of(self.i, root).into_iter().map(|id| Event::Menu { id, checked: crate::menu::checked_of(self.i, root, id), gadget });
        let first = evs.next();
        self.queue.borrow_mut().extend(evs);
        first
    }

    /// The check mark of menu item `id` (the window's menus or a context
    /// menu); None when no menu has it.
    pub fn menu_checked(&self, id: u32) -> Option<bool> {
        self.built.menus.iter().find_map(|m| m.1.checked(id))
    }

    /// Set or clear a menu item's check mark in every menu that has it
    /// (the window's and context menus). False when none has `id`.
    pub fn set_menu_checked(&self, id: u32, on: bool) -> bool {
        self.built.menus.iter().fold(false, |any, m| m.1.set_checked(id, on) || any)
    }

    /// Grey a menu item out, or make it usable. False when no menu has `id`.
    pub fn set_menu_disabled(&self, id: u32, on: bool) -> bool {
        self.built.menus.iter().fold(false, |any, m| m.1.set_disabled(id, on) || any)
    }

    /// The kind of gadget `id`, if the window has one.
    pub fn kind(&self, id: u16) -> Option<GadgetKind> {
        self.built.gadgets.iter().find(|g| g.0 == id).map(|g| g.1)
    }

    fn gadget(&self, id: u16, want: &[GadgetKind]) -> Result<*mut APTR> {
        match self.built.gadgets.iter().find(|g| g.0 == id) {
            Some(&(_, k, o)) if want.contains(&k) => Ok(o),
            Some(_) => Err(AmigaError::InvalidArgument("the gadget is of another kind")),
            None => Err(AmigaError::InvalidArgument("no gadget has that id")),
        }
    }

    fn attr(&self, o: *mut APTR, tag: u32) -> Option<u32> {
        let mut v = 0u32;
        // SAFETY: a live object of this window and a u32 to write.
        let ok = unsafe { ((*self.i).GetAttr)(self.i, tag, o, &mut v) } != 0;
        ok.then_some(v)
    }

    fn get(&self, id: u16, want: &[GadgetKind], tag: u32) -> Result<u32> {
        let o = self.gadget(id, want)?;
        self.attr(o, tag).ok_or(AmigaError::Unsupported("the gadget does not answer that attribute"))
    }

    fn set(&self, id: u16, want: &[GadgetKind], tags: &[TagItem]) -> Result<()> {
        let o = self.gadget(id, want)?;
        // SAFETY: a gadget of this open window and a TAG_DONE-ended list;
        // a gadget that asks for it is redrawn (RefreshGList).
        unsafe {
            let w = self.win.get();
            if ((*self.i).SetGadgetAttrsA)(self.i, o as *mut sys::Gadget, w, core::ptr::null_mut(), tags.as_ptr()) != 0 && !w.is_null() {
                ((*self.i).RefreshGList)(self.i, o as *mut sys::Gadget, w, core::ptr::null_mut(), 1);
            }
        }
        Ok(())
    }

    /// A string gadget's text.
    pub fn text(&self, id: u16) -> Result<String> {
        let p = self.get(id, &[GadgetKind::String], STRINGA_TEXT_VAL)? as *const u8;
        if p.is_null() {
            return Ok(String::new());
        }
        // SAFETY: the gadget's own NUL-terminated buffer, copied now.
        Ok(String::from_utf8_lossy(unsafe { core::ffi::CStr::from_ptr(p as *const core::ffi::c_char) }.to_bytes()).into_owned())
    }

    pub fn set_text(&self, id: u16, text: &str) -> Result<()> {
        let c = c_copy(text.as_bytes())?;
        // The gadget copies the text into its buffer.
        self.set(id, &[GadgetKind::String], &[TagItem { ti_Tag: STRINGA_TEXT_VAL, ti_Data: c.as_ptr() as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// A check box ticked, or a push button down.
    pub fn checked(&self, id: u16) -> Result<bool> {
        Ok(self.get(id, &[GadgetKind::CheckBox, GadgetKind::Button], GA_SELECTED)? != 0)
    }

    pub fn set_checked(&self, id: u16, on: bool) -> Result<()> {
        self.set(id, &[GadgetKind::CheckBox, GadgetKind::Button], &[TagItem { ti_Tag: GA_SELECTED, ti_Data: on as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// An integer gadget's number.
    pub fn number(&self, id: u16) -> Result<i32> {
        Ok(self.get(id, &[GadgetKind::Integer], INTEGER_NUMBER)? as i32)
    }

    pub fn set_number(&self, id: u16, v: i32) -> Result<()> {
        self.set(id, &[GadgetKind::Integer], &[TagItem { ti_Tag: INTEGER_NUMBER, ti_Data: v as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// A slider's or gauge's level.
    pub fn level(&self, id: u16) -> Result<i32> {
        let tag = if self.kind(id) == Some(GadgetKind::Gauge) { FUELGAUGE_LEVEL } else { SLIDER_LEVEL };
        Ok(self.get(id, &[GadgetKind::Slider, GadgetKind::Gauge], tag)? as i32)
    }

    pub fn set_level(&self, id: u16, v: i32) -> Result<()> {
        let tag = if self.kind(id) == Some(GadgetKind::Gauge) { FUELGAUGE_LEVEL } else { SLIDER_LEVEL };
        self.set(id, &[GadgetKind::Slider, GadgetKind::Gauge], &[TagItem { ti_Tag: tag, ti_Data: v as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// A chooser's item.
    pub fn active(&self, id: u16) -> Result<u32> {
        self.get(id, &[GadgetKind::Chooser], CHOOSER_ACTIVE)
    }

    pub fn set_active(&self, id: u16, k: u32) -> Result<()> {
        self.set(id, &[GadgetKind::Chooser], &[TagItem { ti_Tag: CHOOSER_ACTIVE, ti_Data: k }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// A list's selected row, None when none is.
    pub fn selected(&self, id: u16) -> Result<Option<u32>> {
        let v = self.get(id, &[GadgetKind::List], LISTBROWSER_SELECTED)? as i32;
        Ok((v >= 0).then_some(v as u32))
    }

    pub fn set_selected(&self, id: u16, row: Option<u32>) -> Result<()> {
        let v = row.map(|r| r as i32).unwrap_or(-1) as u32;
        self.set(id, &[GadgetKind::List], &[TagItem { ti_Tag: LISTBROWSER_SELECTED, ti_Data: v }, TagItem { ti_Tag: LISTBROWSER_MAKE_VISIBLE, ti_Data: v }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// Replace a list's rows (none selected afterwards).
    pub fn set_items(&mut self, id: u16, items: &[&str]) -> Result<()> {
        let o = self.gadget(id, &[GadgetKind::List])?;
        let items: Vec<String> = items.iter().map(|s| String::from(*s)).collect();
        let lb = lb_iface()?;
        // Detach the old list (~0), make and attach the new one, then free
        // the old.
        // SAFETY: a list gadget of this open window; ~0 detaches its list.
        unsafe { ((*self.i).SetGadgetAttrsA)(self.i, o as *mut sys::Gadget, self.win.get(), core::ptr::null_mut(), [TagItem { ti_Tag: LISTBROWSER_LABELS, ti_Data: !0 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }].as_ptr()) };
        let old = self.built.lists.iter().position(|l| l.0 == id).map(|k| self.built.lists.remove(k));
        let new = self.built.list(id, &items);
        let attach = match &new {
            Ok(lp) => *lp as u32,
            Err(_) => 0,
        };
        self.set(id, &[GadgetKind::List], &[TagItem { ti_Tag: LISTBROWSER_LABELS, ti_Data: attach }, TagItem { ti_Tag: LISTBROWSER_SELECTED, ti_Data: !0 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])?;
        if let Some((_, mut l)) = old {
            // SAFETY: the detached list, which no gadget shows any more.
            unsafe { ((*lb).FreeListBrowserList)(lb, l.as_mut_ptr() as *mut sys::List) };
        }
        new.map(|_| ())
    }

    /// A border scroller object (window.class makes and owns it).
    fn scroller(&self, vertical: bool) -> Option<*mut APTR> {
        self.scroll.as_ref()?;
        let o = self.attr(self.obj, if vertical { WINDOW_VERT_OBJECT } else { WINDOW_HORIZ_OBJECT })?;
        (o != 0).then_some(o as *mut APTR)
    }

    /// A scroller whose knob the user moved since last seen.
    fn scroll_moved(&self) -> Option<Event> {
        let cell = self.scroll.as_ref()?;
        let mut s = cell.get();
        for (k, vertical) in [(0usize, false), (1, true)] {
            let Some(o) = self.scroller(vertical) else { continue };
            let Some(top) = self.attr(o, PGA_TOP) else { continue };
            if top != s[k].2 {
                s[k].2 = top;
                cell.set(s);
                return Some(Event::Scroll { vertical, top });
            }
        }
        None
    }

    /// Set a border scroller: `total` units, `visible` of them shown,
    /// the first shown at `top` (pixels, rows, ...). Only changed values
    /// are sent.
    pub fn set_scroll(&self, vertical: bool, total: u32, visible: u32, top: u32) {
        let (Some(cell), Some(o)) = (self.scroll.as_ref(), self.scroller(vertical)) else { return };
        let mut s = cell.get();
        let k = vertical as usize;
        let top = top.min(total.saturating_sub(visible));
        if s[k] == (total, visible, top) {
            return;
        }
        s[k] = (total, visible, top);
        cell.set(s);
        let tags = [
            TagItem { ti_Tag: PGA_TOTAL, ti_Data: total },
            TagItem { ti_Tag: PGA_VISIBLE, ti_Data: visible },
            TagItem { ti_Tag: PGA_TOP, ti_Data: top },
            TagItem { ti_Tag: TAG_DONE, ti_Data: 0 },
        ];
        // SAFETY: the window's scroller object, our open window (or NULL
        // while iconified) and a TAG_DONE-ended list; the class redraws it.
        unsafe {
            let w = self.win.get();
            if w.is_null() {
                ((*self.i).SetAttrsA)(self.i, o, tags.as_ptr() as *mut TagItem);
            } else {
                ((*self.i).SetGadgetAttrsA)(self.i, o as *mut sys::Gadget, w, core::ptr::null_mut(), tags.as_ptr());
            }
        }
    }

    /// Press a toolbar's toggle button in (or let it out).
    pub fn set_tool_selected(&self, bar: u16, button: u16, on: bool) -> Result<()> {
        let (o, lp, node) = self.tool(bar, button)?;
        let sb = sb_iface()?;
        // The speedbar's nodes may only change while its list is detached
        // (~0 keeps the display); the list goes back after.
        // SAFETY: a toolbar of this open window, its own list and node.
        unsafe {
            ((*self.i).SetGadgetAttrsA)(self.i, o as *mut sys::Gadget, self.win.get(), core::ptr::null_mut(), [TagItem { ti_Tag: SPEEDBAR_BUTTONS, ti_Data: !0 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }].as_ptr());
            let mut t = [TagItem { ti_Tag: SBNA_SELECTED, ti_Data: on as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
            ((*sb).SetSpeedButtonNodeAttrsA)(sb, node, t.as_mut_ptr());
        }
        self.set(bar, &[GadgetKind::Toolbar], &[TagItem { ti_Tag: SPEEDBAR_BUTTONS, ti_Data: lp as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// Whether a toolbar's toggle button is pressed in.
    pub fn tool_selected(&self, bar: u16, button: u16) -> Result<bool> {
        let (_, _, node) = self.tool(bar, button)?;
        let sb = sb_iface()?;
        let mut v = 0u32;
        let mut t = [TagItem { ti_Tag: SBNA_SELECTED, ti_Data: &mut v as *mut u32 as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }];
        // SAFETY: a node of this window's toolbar and a u32 to write.
        unsafe { ((*sb).GetSpeedButtonNodeAttrsA)(sb, node, t.as_mut_ptr()) };
        Ok(v != 0)
    }

    /// A toolbar's object, its list and a button's node.
    fn tool(&self, bar: u16, button: u16) -> Result<(*mut APTR, *mut sys::List, *mut sys::Node)> {
        let o = self.gadget(bar, &[GadgetKind::Toolbar])?;
        let t = self.built.toolbars.iter().find(|t| t.id == bar).ok_or(AmigaError::InvalidArgument("no toolbar has that id"))?;
        let node = t.nodes.iter().find(|n| n.0 == button).map(|n| n.1).ok_or(AmigaError::InvalidArgument("the toolbar has no button with that id"))?;
        Ok((o, t.list.as_ptr() as *mut sys::List, node))
    }

    /// Grey a gadget out (or make it usable again).
    pub fn set_disabled(&self, id: u16, on: bool) -> Result<()> {
        let all = [GadgetKind::Button, GadgetKind::String, GadgetKind::CheckBox, GadgetKind::Integer, GadgetKind::Slider, GadgetKind::Chooser, GadgetKind::List, GadgetKind::Gauge, GadgetKind::Space, GadgetKind::Toolbar];
        self.set(id, &all, &[TagItem { ti_Tag: GA_DISABLED, ti_Data: on as u32 }, TagItem { ti_Tag: TAG_DONE, ti_Data: 0 }])
    }

    /// Whether a gadget is greyed out.
    pub fn disabled(&self, id: u16) -> Result<bool> {
        let all = [GadgetKind::Button, GadgetKind::String, GadgetKind::CheckBox, GadgetKind::Integer, GadgetKind::Slider, GadgetKind::Chooser, GadgetKind::List, GadgetKind::Gauge, GadgetKind::Space, GadgetKind::Toolbar];
        Ok(self.get(id, &all, GA_DISABLED)? != 0)
    }
}

impl crate::intuition::ParentWindow for Window<'_> {
    fn window_ptr(&self) -> *mut sys::Window {
        self.win.get()
    }
}

/// A toolbar's button list (boxed: the speedbar keeps its address) and
/// its buttons' nodes by id.
struct Toolbar {
    id: u16,
    list: Box<[u32; 4]>,
    nodes: Vec<(u16, *mut sys::Node)>,
}

fn free_lists(b: &mut Built) {
    if !b.toolbars.is_empty() {
        if let Ok(sb) = sb_iface() {
            for l in b.toolbars.iter_mut() {
                let lp = l.list.as_mut_ptr() as *mut sys::List;
                // SAFETY: our speedbar node list, which no gadget shows any
                // more: each node is taken off (RemHead) and freed once.
                unsafe {
                    loop {
                        let n = sys::exec_rem_head(lp);
                        if n.is_null() {
                            break;
                        }
                        ((*sb).FreeSpeedButtonNode)(sb, n);
                    }
                }
            }
        }
        b.toolbars.clear();
    }
    if let Ok(i) = crate::intuition::intuition_iface() {
        for o in b.images.drain(..) {
            // SAFETY: a bitmap.image of ours, no longer shown by a speedbar.
            unsafe { ((*i).DisposeObject)(i, o) };
        }
    }
    if b.lists.is_empty() {
        return;
    }
    if let Ok(lb) = lb_iface() {
        for (_, l) in b.lists.iter_mut() {
            // SAFETY: a node list of ours that no gadget shows any more
            // (its gadget is disposed or was never made).
            unsafe { ((*lb).FreeListBrowserList)(lb, l.as_mut_ptr() as *mut sys::List) };
        }
    }
    b.lists.clear();
}

impl Drop for Window<'_> {
    fn drop(&mut self) {
        // The window object closes the window and disposes the layout,
        // which disposes the gadgets and label images; then the lists the
        // list browsers showed are freed, then the texts (Built drops).
        dispose(self.i, self.obj);
        free_lists(&mut self.built);
        if let Some(p) = &self.port {
            while let Some(m) = p.get_raw() {
                // SAFETY: a message left on our port (the window object that
                // used it is gone), replied once.
                unsafe { sys::exec_reply_msg(m) };
            }
        }
    }
}
