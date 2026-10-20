interface Props { safe: string; }
export default function Root(props: Props) {
    return <button onClick={new Widget(props.typo)}>click</button>;
}
