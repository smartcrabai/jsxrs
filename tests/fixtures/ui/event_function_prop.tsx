interface Props { safe: string; }
export default function Root(props: Props) {
    return <button onClick={function() { return props.typo; }}>click</button>;
}
