use eframe::egui;

// `fn main()` defines the main function, the entry point of every Rust executable.
// `-> Result<(), eframe::Error>` specifies the return type.
// `Result` is a standard Rust enum used for error handling. It can be either:
//  - `Ok(value)`: The operation succeeded, containing the value (here `()`, the empty tuple or "unit type", signifying no specific value).
//  - `Err(error_value)`: The operation failed, containing an error value (here, an `eframe::Error`).
// This means `main` can signal if it failed to start the eframe application.
fn main() -> Result<(), eframe::Error> {
    // `let options = ...;` declares a variable named `options`.
    // `eframe::NativeOptions::default()` calls an "associated function" (like a static method)
    // named `default` on the `NativeOptions` struct within the `eframe` crate.
    // The `Default` trait provides this standard way to get default values.
    let options = eframe::NativeOptions::default();

    // Call the `run_native` function from the `eframe` crate.
    eframe::run_native(
        "egui Demo", // Window title (a string literal)
        options,     // The options struct we just created
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

// `#[derive(Default)]` is an "attribute" that asks the compiler to automatically
// generate a default implementation for this struct. For `MyApp`, this means
// creating an instance where `label` is an empty `String` and `value` is `0.0`.
// We will modify this struct and its Default implementation later.
#[derive(Default)]
struct MyApp {
    // `label: String,` defines a field named `label` of type `String` (a growable text string).
    label: String,
    // `value: f32,` defines a field named `value` of type `f32` (a 32-bit floating-point number).
    value: f32,
    // We'll add more fields later!
}

// `eframe::App` requires structs used with `run_native` to have methods like `update`.
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // `egui::CentralPanel::default()` creates a default central panel configuration.
        // `.show(ctx, |ui| { ... })` calls the `show` method on the panel.
        egui::CentralPanel::default().show(ctx, |ui| {
            // `ui` is of type `&mut egui::Ui`. It's a mutable reference, so methods called on `ui` can change its internal state (e.g., layout position).
            // `ui.heading(...)` calls the `heading` method on the `ui` object.
            ui.heading("My egui Application");

            // `ui.horizontal(|ui| { ... });` uses another closure for horizontal layout.
            ui.horizontal(|ui| {
                ui.label("Write something: ");
                // `ui.text_edit_singleline(&mut self.label);`
                //   `&mut self.label`: Provides a *mutable reference* to the `label` field of our `MyApp` instance (`self`).
                //   This allows the `text_edit_singleline` widget to *directly modify* the `label` field in our state
                //   when the user types into the text box. This is fundamental to egui's state handling.
                ui.text_edit_singleline(&mut self.label);
            });

            // `ui.add(...)` is a general method to add any widget.
            // `egui::Slider::new(&mut self.value, 0.0..=10.0)` creates a slider widget configuration.
            //   `&mut self.value`: Mutably borrows the `value` field from `MyApp`.
            //   `0.0..=10.0`: Defines the range (inclusive) for the slider using Rust's range syntax.
            // `.text("value")`: A builder method to add a label next to the slider.
            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));

            // `if ui.button("Increment").clicked() { ... }`
            //   `ui.button("Increment")`: Creates a button widget and returns a `Response` struct.
            //   `.clicked()`: Calls the `clicked` method on the `Response`. It returns `true` if the button was clicked in this frame, `false` otherwise.
            //   `if ... { ... }`: If `clicked()` is true, execute the code block.
            if ui.button("Increment").clicked() {
                // `self.value += 1.0;`
                //   Accesses the `value` field of our `MyApp` instance (`self`) and increases it by 1.0.
                //   Because `update` has `&mut self`, we are allowed to modify the fields.
                self.value += 1.0;
            }

            // `ui.label(format!(...));` Adds a label.
            // `format!("Hello '{}', value: {}", self.label, self.value)`: A macro to create a formatted `String`.
            //   `{}` are placeholders. `self.label` and `self.value` provide the values to insert.
            //   It reads the *current* state of `self.label` and `self.value` for display.
            ui.label(format!("Hello '{}', value: {}", self.label, self.value));
        });
    }
}
