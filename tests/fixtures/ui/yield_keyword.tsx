interface YieldProps { yield: string; }

export default function Page(props: YieldProps) {
    return <main>{props.yield}</main>;
}
