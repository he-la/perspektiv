# Contributing

Pull requests are very welcome! Just fork the repository, make your changes, and
commit as you go. To submit your PR, please do the following:
1. Ensure you haven't included temporary files in your commit
2. Run `cargo fmt` (you'll need rustfmt for this)
3. Push!

## Project Structure

"Modules" are the basic building blocks of perspektiv. A module takes care of
polling for some events and translating that information into messages for the
UI to display. Each module is run in its own thread. Threading is abstracted
away through two main mechanisms: The `threlm` UI library in `./threlm`, and the
`Subscribable` trait.

This trait bridges the UI with the modules by requiring modules implementing the
trait to expose a function `poll_factory`, and offers a function `subscribe` to
the calling modules. Here's how these functions work together:
1. The UI calls `subscribe` on a module. This is the function provided by the
   trait.
2. The `subscribe` function spawns a new thread with the name of the module, and
   runs the `poll_factory` function with the arguments provided to `subscribe`
   to obtain a closure from the module (or an error if the closure could not be
   created).
3. The closure is run in a loop, and expected to return a `Result<<Message>,
   Error>`. The closure should take care of polling for events, and then return
   an appropriate `ui::Msg` message to the UI, or an error of type
   `subscribable::Error`.
   
To write a new module, you must
- Write the code for the module using the `Subscribable` trait
- Subscribe to the module by adding the following code to the end of the
  `connect` method in `./src/ui.rs`:
  ```rust
  #[cfg(feature = "your_module_name")]
  subsribe!(your_module_name, your_module_options)
  ```
- Add your module to the end of the section marked by `// MODULES` in
  `./src/main.rs` and add it to `#[cfg(not(any(feature = ...)))]` list bellow
  the modules section.
