# Threlm

Threlm is the UI library that drives perspektiv. I have segragated it into its
own little library because I hope to release it indepedentently in the future.
Threlm is loosely inspired by the excellent
[relm](https://github.com/antoyo/relm) library. It was build out of necessity
for a simple, thread-safe GTK wrapper for rust. At the time of threlm's
inception, relm was neither of those things.

Currently, threlm is really just a quick hack to implement a simple actor model
for asynchronous computation along with some bare functionality for integrating
that with GTK widgets. It does not have any of the syntactic sugar and extended
features of relm, and I'm a bit on the fence as to whether I want to add those
over just keeping it simple.

The core of threlm relies on sharing weak references to actors (called models as
per relm) across thread boundaries. When a thread wishes to send a message to a
model, a closure is injected into the glib event loop to call the update method
on the model with the supplied message as its argument. If the model was already
deallocated, an error is returned such that the sending thread may free any
resources related to the connection with the model.

A drawback of this method is the large amount of atomic operations: When cloning
(sharing) weak references, a reference counter to the model must be atomically
increased. Since every message emission invokes a clone on the weak reference,
there is some overhead in first synchronising with the main thread to increase
the reference count, and later synchronising again to pass the closure.

It might therefore be benefical to switch to the rust-native mpsc channels.
These channels are presumably better optimised. However, they require waking the
glib loop when a message is sent, which relies on linux's eventfd2 syscall. This
might also incur a comparable overhead.

For now, I choose not to worry too much about this minor inefficiency.
