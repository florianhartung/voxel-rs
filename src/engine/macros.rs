//! Shorthand for matching winit keyboard press events
//!
//! # Example
//! ## Without macro
//! ```rust
//! let event: Event<()>;
//!
//! match event {
//!     Event::WindowEvent {
//!         event: WindowEvent::KeyboardInput {
//!             input: KeyboardInput {
//!                 virtual_keycode: Some(VirtualKeyCode::Escape),
//!                 state: ElementState::Pressed,
//!                 ..
//!             },
//!             ..
//!         },
//!         ..
//!     } => { println!("Escape was pressed!"); }
//!     _ => {}
//! }
//! ```
//!
//! ## With macro
//! ```rust
//! let event: Event<()>;
//!
//! match event {
//!     matches_key_press!(VirtualKeyCode::Escape) => { println!("Escape was pressed!"); }
//!     _ => {}
//! }
//! ```
macro_rules! key_press {
    ( $x:path ) => {
        winit::event::Event::WindowEvent {
            event: winit::event::WindowEvent::KeyboardInput {
                input: winit::event::KeyboardInput {
                    virtual_keycode: Some($x),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            },
            ..
        }
    }
}

macro_rules! close_requested {
    () => {
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        }
    }
}