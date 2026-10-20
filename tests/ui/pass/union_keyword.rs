jsxrs::component!(pub union_keyword, "tests/fixtures/ui/union_keyword.tsx");

fn main() {
    let _props = union_keyword::Props {
        r#union: String::from("value"),
    };
}
