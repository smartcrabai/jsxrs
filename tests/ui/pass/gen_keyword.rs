jsxrs::component!(pub gen_keyword, "tests/fixtures/ui/gen_keyword.tsx");

fn main() {
    let _props = gen_keyword::Props {
        r#gen: String::from("value"),
    };
}
