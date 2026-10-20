interface LayoutProps { locale?: string; children: string; }
export default function Layout(props: LayoutProps) { return <html><body><p>{props.locale}</p>{props.children}</body></html>; }
