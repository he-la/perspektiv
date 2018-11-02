// This file is part of perspektiv, a userspace daemon for graphically reporting
// system events.
// Copyright © 2018  Henrik Laxhuber <henrik@laxhuber.com>
//
// perspektiv is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License, version 3, as published by the
// Free Software Foundation.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

extern crate glib;
extern crate gtk;

use std::{clone::Clone, ops::Deref, sync, sync::Arc};

use gtk::{Continue, IsA};

struct UnsafeMutCell<T: Sized>(T);
unsafe impl<T: Sized> Send for UnsafeMutCell<T> {}
unsafe impl<T: Sized> Sync for UnsafeMutCell<T> {}
impl<T: Sized> Deref for UnsafeMutCell<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

// TODO: Consider implementing monadic apply_mut(&self, f) to make it impossible
// for two mutable references to coexist. Right now there is no great reason to
// do so as the library is very small and simple, but this might change in the
// future.
impl<T: Sized> UnsafeMutCell<T> {
    pub unsafe fn borrow_mut(&self) -> &mut T {
        &mut *(&self.0 as *const T as *mut T)
    }
}

/// Owns a struct implementing the [`Model`] trait and allows [`Actor`]s to
/// be created from it. Actors can be passed around thread boundaries and
/// are used to send messages to the contained struct.
///
/// Creating a Threlm object will call the [`Model::connect`] method on the contained
/// model. This means that a model on its own cannot receive messages; you must
/// first move it into a Threlm object:
/// ```rust
/// let model = Model::new(foo);
/// let model_threlm = Threlm::new(model); // <- calls model.connect
/// ```
///
/// You must keep this object in scope for as long as you want the contained
/// model to receive messages.
// This architecture is a monadic implementation of the actor model. Since the
// Threlm struct itself is neither Sync nor Send, it ensures that it remains on
// the thread where it was created. An assertion ensures that this is the glib
// main thread. Actor structs (weak references) likewise ensure that any
// operations are performed on the glib main thread.
//
// It might make a lot of sense to port this code to use rusts mpsc channels.
// I'm very much on the fence about this, as the number of atomic operations
// should generally be about the same.
pub struct Threlm<C: Model> {
    inner: Arc<UnsafeMutCell<C>>,
}

impl<C: Model + 'static> Threlm<C> {
    #[inline]
    pub fn new(inner: C) -> Self {
        // Asserts that the object
        assert!(gtk::is_initialized_main_thread());

        let actor = Self {
            inner: Arc::new(UnsafeMutCell(inner)),
        };
        actor.inner.connect(actor.actor());

        actor
    }

    /// Obtain a weak reference to the contained model that can be shared across
    /// threads to send messages to it.
    #[inline]
    pub fn actor(&self) -> Actor<C> {
        Actor::new(Arc::downgrade(&self.inner))
    }

    /// *Synchronously* send the contained model a message
    ///
    /// This is safe since `Threlm`s only live in the main thread. Prefer
    /// this method whenever possible (such as updating child actors from
    /// the parent), as it is faster.
    #[inline]
    pub fn update(&mut self, message: C::Message) {
        unsafe { self.inner.borrow_mut() }.update(message, self.actor());
    }
}

/// Weak reference to a [`Threlm`] object that can send messages to the
/// contained object.
pub struct Actor<C: Model + 'static> {
    inner: sync::Weak<UnsafeMutCell<C>>,
}

impl<C: Model + 'static> Actor<C> {
    #[inline]
    fn new(inner: sync::Weak<UnsafeMutCell<C>>) -> Self {
        Self { inner }
    }

    /// Attempts to send a message to this actor.
    ///
    /// An error is returned if the referenced model has been deallocated. Note
    /// that a return value of Ok does not mean that the model received the
    /// message. It is possible for the model to be deallocated after this
    /// function returns Ok, but beffore the message is actually received on the
    /// GTK main thread.
    ///
    /// You can disconnect from the contained model on the next call to tell(),
    /// which will then yield an error.
    //  - TODO: Consider creating a custom error enum. I'm not sure if this is
    //  useful though, as there is only one possible cause for error here.
    //  - TODO: Create some tell_ensure that blocks until the message has been
    // received, e.g. using a condvar.
    pub fn tell(&self, message: C::Message) -> Result<(), &'static str> {
        // TODO: There should be a more efficient way to simply check if the
        // strong reference to a sync::Weak is still valid. Currently, in the
        // standard library, there is not.
        if self.inner.upgrade().is_none() {
            return Err("The referenced model has already been deallocated");
        }
        let this = self.clone();
        glib::idle_add(move || {
            let message = message.clone();
            if let Some(model) = this.inner.upgrade() {
                unsafe { model.borrow_mut() }.update(message, this.clone());
            }
            Continue(false)
        });

        Ok(())
    }
}

impl<C: Model> Clone for Actor<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

unsafe impl<C: Model> Send for Actor<C> {}
unsafe impl<C: Model> Sync for Actor<C> {}

/// TODO: Document
pub trait Model: Sized {
    /// Type of the messages that this model can receive.
    type Message: Send + Clone;

    fn connect(&self, stream: Actor<Self>);

    fn update(&mut self, message: Self::Message, stream: Actor<Self>);
}

/// TODO: Document
pub trait View: Model {
    type Root: IsA<gtk::Widget>;

    fn root(&self) -> &Self::Root;
}

// =======================

// use self::Msg::*;
//
// #[derive(Clone, Debug)]
// enum Msg {
//     Quit,
// }
//
// struct App {
//     window: gtk::Window,
//     button: gtk::Button,
// }
//
// impl App {
//     pub fn new() -> Self {
//         let window = gtk::Window::new(Popup);
//
//         let button = gtk::Button::new_with_label("Quit");
//
//         window.add(&button);
//         window.show_all();
//
//         Self { window, button }
//     }
// }
//
// impl Model for App {
//     type Message = Msg;
//
//     // TODO: Find a better name for `this`
//     fn connect(&self, this: Actor<Self>) {
//         self.window.connect_delete_event({
//             let this = this.clone();
//             move |_, _| {
//                 this.tell(Quit).unwrap();
//                 Inhibit(false)
//             }
//         });
//
//         self.button.connect_clicked({
//             let this = this.clone();
//             let button = self.button.clone();
//             move |_| {
//                 let this = this.clone();
//                 button.set_label("Quitting...");
//                 thread::spawn(move || {
//                     // For timers, native glib timeouts would be preferable.
//                     // This is just to provide an example for thread-safe
//                     // messaging in threlm.
//                     thread::sleep(Duration::from_secs(1));
//                     this.tell(Quit).unwrap();
//                 });
//             }
//         });
//     }
//
//     fn update(&mut self, message: Msg) {
//         println!("Got message: {:#?}", message);
//         match message {
//             Quit => {
//                 gtk::main_quit();
//             }
//         }
//     }
// }
//
// impl View for App {
//     type Root = gtk::Window;
//
//     fn root(&self) -> &Self::Root {
//         &self.window
//     }
// }
//
// fn main() {
//     gtk::init().expect("Failed to initialise GTK");
//
//     let app = App::new();
//     let _threlm = Threlm::new(app);
//
//     gtk::main();
// }
