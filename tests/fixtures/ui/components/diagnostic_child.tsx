interface ChildProps { name: string; }
export default function Child(props: ChildProps) { return <p>{props.unknown}</p>; }
