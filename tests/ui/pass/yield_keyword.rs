jsxrs::component!(pub yield_keyword, "tests/fixtures/ui/yield_keyword.tsx");

fn main() {
    let _props = yield_keyword::Props {
        r#yield: String::from("value"),
    };
}
