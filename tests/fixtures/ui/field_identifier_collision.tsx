interface Props { "foo-bar": string; field_0: string; }
export default function Page(props: Props) { return <p>{props["foo-bar"]}{props.field_0}</p>; }
