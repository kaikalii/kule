use std::collections::HashSet;

use glutin::event::{self, *};
use vector2math::*;

use crate::{Camera, Vec2};

pub use event::ElementState as ButtonState;
pub use event::ModifiersState as Modifiers;
pub use event::MouseButton;

/// An input event
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "ser",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum Event {
    /// The mouse cursor's absolute position has changed
    MouseAbsolute(Vec2),
    /// The mouse cursor's relative position has changed
    MouseRelative(Vec2),
    /// A mouse button's state has changed
    MouseButton {
        /// The mouse button
        button: MouseButton,
        /// The new state
        state: ButtonState,
    },
    /// A key's state has changed
    Key {
        /// The key
        key: Key,
        /// The scancode
        scancode: u32,
        /// The new state
        state: ButtonState,
    },
    /// The window was resized
    Resize(Vec2),
    /// The window was moved
    Move(Vec2),
    /// The window has gained or lost focus
    Focus(bool),
    /// The mouse wheel was scrolled
    Scroll(Vec2),
    /// The window was requested to close
    CloseRequest,
}

impl Event {
    pub(crate) fn from_glutin(
        event: event::Event<()>,
        tracker: &mut StateTracker,
        camera: &mut Camera,
    ) -> Two<Self> {
        let window_event = if let event::Event::WindowEvent { event, .. } = event {
            event
        } else {
            return Two::none();
        };
        match window_event {
            WindowEvent::CloseRequested => Event::CloseRequest.into(),
            WindowEvent::Resized(size) => {
                let size = [size.width as f32, size.height as f32];
                camera.window_size = size;
                Event::Resize(size).into()
            }
            WindowEvent::Moved(size) => Event::Move([size.x as f32, size.y as f32]).into(),
            WindowEvent::Focused(foc) => Event::Focus(foc).into(),
            WindowEvent::CursorMoved { position, .. } => {
                let pos = [position.x as f32, position.y as f32];
                let two = Two::two(
                    Event::MouseAbsolute(pos),
                    Event::MouseRelative(pos.sub(tracker.mouse_pos)),
                );
                tracker.mouse_pos = pos;
                two
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ButtonState::Pressed => tracker.mouse_buttons.insert(button),
                    ButtonState::Released => tracker.mouse_buttons.remove(&button),
                };
                Event::MouseButton { button, state }.into()
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(x, y),
                ..
            } => Event::Scroll([x, y]).into(),
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(pos),
                ..
            } => Event::Scroll([pos.x as f32, pos.y as f32]).into(),
            WindowEvent::ModifiersChanged(modifiers) => {
                tracker.modifiers = modifiers;
                Two::none()
            }
            WindowEvent::KeyboardInput { input, .. } => {
                let key = input
                    .virtual_keycode
                    .map(Key::from_glutin)
                    .unwrap_or(Key::Unknown);
                match input.state {
                    ButtonState::Pressed => tracker.keys.insert(key),
                    ButtonState::Released => tracker.keys.remove(&key),
                };
                Event::Key {
                    key,
                    scancode: input.scancode,
                    state: input.state,
                }
                .into()
            }
            _ => Two::none(),
        }
    }
}

/**
Tracks various input states

The context updates its `StateTracker` automatically.
*/
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "ser",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct StateTracker {
    mouse_pos: Vec2,
    modifiers: Modifiers,
    keys: HashSet<Key>,
    mouse_buttons: HashSet<MouseButton>,
    pub(crate) fps: f32,
}

impl StateTracker {
    /// Get the position of the mouse cursor in window space
    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }
    /// Get the state of modifier keys
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
    /// Get the state of a key
    pub fn key(&self, key: Key) -> bool {
        self.keys.contains(&key)
    }
    /// Get the state of a mouse button
    pub fn mouse_button(&self, mb: MouseButton) -> bool {
        self.mouse_buttons.contains(&mb)
    }
    /**
    Get a scalar representing the difference between two key states

    This is useful for times when you need two keys to represent different
    directions of some control, i.e. zooming in and out with +-.
    */
    pub fn key_diff_scalar(&self, neg: Key, pos: Key) -> f32 {
        self.key(pos) as i8 as f32 - self.key(neg) as i8 as f32
    }
    /**
    Get a vector representing the difference between two pairs of key states

    This is useful for times when you need four keys to represent different
    directions of some control, i.e. controlling a character with WASD.
    */
    pub fn key_diff_vector(&self, left: Key, right: Key, up: Key, down: Key) -> Vec2 {
        [
            self.key_diff_scalar(left, right),
            self.key_diff_scalar(up, down),
        ]
    }
    /// Get the temporally-normalized frames per second
    pub fn fps(&self) -> f32 {
        self.fps
    }
}

pub(crate) struct Two<T>(Option<T>, Option<T>);

impl<T> Two<T> {
    pub const fn none() -> Self {
        Two(None, None)
    }
    pub const fn one(item: T) -> Self {
        Two(Some(item), None)
    }
    pub const fn two(one: T, two: T) -> Self {
        Two(Some(one), Some(two))
    }
}

impl<T> From<T> for Two<T> {
    fn from(item: T) -> Self {
        Two::one(item)
    }
}

impl<T> Iterator for Two<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().or_else(|| self.1.take())
    }
}

macro_rules! keys {
    ($(($key:ident, $glutinkey:ident),)*) => {
        #[allow(missing_docs)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(
            feature = "ser",
            derive(serde_derive::Serialize, serde_derive::Deserialize)
        )]
        pub enum Key {
            $($key,)*
            Unknown
        }

        impl Key {
            fn from_glutin(key: event::VirtualKeyCode) -> Self {
                match key {
                    $(event::VirtualKeyCode::$glutinkey => Key::$key),*
                }
            }
        }
    };
}

keys!(
    (Num1, Key1),
    (Num2, Key2),
    (Num3, Key3),
    (Num4, Key4),
    (Num5, Key5),
    (Num6, Key6),
    (Num7, Key7),
    (Num8, Key8),
    (Num9, Key9),
    (Num0, Key0),
    (A, A),
    (B, B),
    (C, C),
    (D, D),
    (E, E),
    (F, F),
    (G, G),
    (H, H),
    (I, I),
    (J, J),
    (K, K),
    (L, L),
    (M, M),
    (N, N),
    (O, O),
    (P, P),
    (Q, Q),
    (R, R),
    (S, S),
    (T, T),
    (U, U),
    (V, V),
    (W, W),
    (X, X),
    (Y, Y),
    (Z, Z),
    (Escape, Escape),
    (F1, F1),
    (F2, F2),
    (F3, F3),
    (F4, F4),
    (F5, F5),
    (F6, F6),
    (F7, F7),
    (F8, F8),
    (F9, F9),
    (F10, F10),
    (F11, F11),
    (F12, F12),
    (F13, F13),
    (F14, F14),
    (F15, F15),
    (F16, F16),
    (F17, F17),
    (F18, F18),
    (F19, F19),
    (F20, F20),
    (F21, F21),
    (F22, F22),
    (F23, F23),
    (F24, F24),
    (Snapshot, Snapshot),
    (Scroll, Scroll),
    (Pause, Pause),
    (Insert, Insert),
    (Home, Home),
    (Delete, Delete),
    (End, End),
    (PageDown, PageDown),
    (PageUp, PageUp),
    (Left, Left),
    (Up, Up),
    (Right, Right),
    (Down, Down),
    (Back, Back),
    (Enter, Return),
    (Space, Space),
    (Compose, Compose),
    (Caret, Caret),
    (Numlock, Numlock),
    (Numpad0, Numpad0),
    (Numpad1, Numpad1),
    (Numpad2, Numpad2),
    (Numpad3, Numpad3),
    (Numpad4, Numpad4),
    (Numpad5, Numpad5),
    (Numpad6, Numpad6),
    (Numpad7, Numpad7),
    (Numpad8, Numpad8),
    (Numpad9, Numpad9),
    (AbntC1, AbntC1),
    (AbntC2, AbntC2),
    (Add, Add),
    (Apostrophe, Apostrophe),
    (Apps, Apps),
    (At, At),
    (Ax, Ax),
    (Backslash, Backslash),
    (Calculator, Calculator),
    (Capital, Capital),
    (Colon, Colon),
    (Comma, Comma),
    (Convert, Convert),
    (Decimal, Decimal),
    (Divide, Divide),
    (Equals, Equals),
    (Grave, Grave),
    (Kana, Kana),
    (Kanji, Kanji),
    (LAlt, LAlt),
    (LBracket, LBracket),
    (LControl, LControl),
    (LShift, LShift),
    (LWin, LWin),
    (Mail, Mail),
    (MediaSelect, MediaSelect),
    (MediaStop, MediaStop),
    (Minus, Minus),
    (Multiply, Multiply),
    (Mute, Mute),
    (MyComputer, MyComputer),
    (NavigateForward, NavigateForward),
    (NavigateBackward, NavigateBackward),
    (NextTrack, NextTrack),
    (NoConvert, NoConvert),
    (NumpadComma, NumpadComma),
    (NumpadEnter, NumpadEnter),
    (NumpadEquals, NumpadEquals),
    (OEM102, OEM102),
    (Period, Period),
    (PlayPause, PlayPause),
    (Power, Power),
    (PrevTrack, PrevTrack),
    (RAlt, RAlt),
    (RBracket, RBracket),
    (RControl, RControl),
    (RShift, RShift),
    (RWin, RWin),
    (Semicolon, Semicolon),
    (Slash, Slash),
    (Sleep, Sleep),
    (Stop, Stop),
    (Subtract, Subtract),
    (Sysrq, Sysrq),
    (Tab, Tab),
    (Underline, Underline),
    (Unlabeled, Unlabeled),
    (VolumeDown, VolumeDown),
    (VolumeUp, VolumeUp),
    (Wake, Wake),
    (WebBack, WebBack),
    (WebFavorites, WebFavorites),
    (WebForward, WebForward),
    (WebHome, WebHome),
    (WebRefresh, WebRefresh),
    (WebSearch, WebSearch),
    (WebStop, WebStop),
    (Yen, Yen),
    (Copy, Copy),
    (Paste, Paste),
    (Cut, Cut),
);
