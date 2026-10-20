jsxrs::component!(pub merged_interface, "tests/fixtures/ui/merged_interface.tsx");

fn main() {
    let props = merged_interface::Props {
        first: String::from("first declaration"),
        second: 2.0,
    };
    let _first = props.first;
}
