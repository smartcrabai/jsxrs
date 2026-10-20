jsxrs::component!(pub exported_interface, "tests/fixtures/ui/exported_interface.tsx");

fn main() {
    let _props = exported_interface::Props {
        name: String::from("Ada"),
    };
}
