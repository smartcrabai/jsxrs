interface PageProps { slug: string; }
export default function BlogPage(props: PageProps) { return <p>{props.slug}</p>; }
