# Contributing

Pull requests are very welcome! Just fork the repository, make your changes, and
commit as you go. To submit your PR, please do the following:
1. Ensure you haven't included temporary files in your commit
2. Run `cargo fmt` (you'll need rustfmt for this)
3. Push!

## Project Structure

In perspektiv, modules provide the underlying functionality of listening for
system events and reporting them to the UI. Modules are basically just rust
files in the `./src` directory. Each module is run in its own thread, and is
expected to `poll` for its events, i.e. block until an event is received. The
module should then sends a message to the UI thread, which takes care of
displaying the information in the popup.

Inter-thread communication is abstracted away into the `Subscribable` trait and
the `threlm` UI library in `./threlm`. The `Subscribable` trait provides the
`subscribe` function to the UI. It requires modules implementing the trait to
expose a function `poll_factory`. Here's how they work together:
1. The UI calls `subscribe` on a module. This is the function provided by the
   trait.
2. The `subscribe` function spawns a new thread with the name of the module, and
   runs the `poll_factory` function to obtain a closure from the module (or an
   error).
3. The closure is run in a loop, and expected to return a
   `Result<Option<Message>, Error>`. This means that the closure returns either
   nothing (the event received turned out to be unusable or was ignored for some
   other reason), or a message for the UI, or an error. The type of the error is
   the error struct defined in `subscribable::Error`. The type of the message is
   `ui::Msg`.
   
   This pattern allows thread spawning and error handling to be abstracted away
   into the Subscribable trait, at the cost of making the `poll_factory`
   function rather opaque.
