interface PageProps { locale?: string; }
export default function Page(props: PageProps) { return <p>{props.locale}</p>; }
