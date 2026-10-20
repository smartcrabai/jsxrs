interface Props { safe: string; }
export default function Root(props: Props) {
    return <button onClick={() => { const copied = props; return copied.typo; }}>click</button>;
}
