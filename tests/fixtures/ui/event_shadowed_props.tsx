interface Props { safe: string; }
export default function Root(props: Props) {
    return <button onClick={(props) => props.typo}>click</button>;
}
